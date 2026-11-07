// SPDX-License-Identifier: Apache-2.0
//! Synthetic command results under real held handles and canonical authority.
use super::*;
use agentmage_kernel_engine::command_runner::{
    CommandBounds, CommandExecutionFailure, CommandLaunchPermit, CommandPlatformResult,
    CommandRisk, CommandTermination,
};
use agentmage_kernel_engine::propagation::EffectCancellationObservation;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn passive_command_poll_retries_only_unavailability_and_keeps_a_fixed_fail_closed_bound() {
    struct Observation {
        calls: AtomicUsize,
        unavailable: usize,
        terminal: std::result::Result<
            Option<agentmage_kernel_contracts::CancellationSignal>,
            CancellationObservationError,
        >,
    }
    impl EffectCancellationObservation for Observation {
        fn observe_effect_cancellation(
            &self,
        ) -> std::result::Result<
            Option<agentmage_kernel_contracts::CancellationSignal>,
            CancellationObservationError,
        > {
            if self.calls.fetch_add(1, Ordering::Relaxed) < self.unavailable {
                Err(CancellationObservationError::Unavailable)
            } else {
                self.terminal.clone()
            }
        }
    }
    for (unavailable, terminal, expected_calls, expected) in [
        (0, Ok(None), 1, Ok(None)),
        (1, Ok(None), 2, Ok(None)),
        (7, Ok(None), 8, Ok(None)),
        (
            8,
            Ok(None),
            8,
            Err(CancellationObservationError::Unavailable),
        ),
        (
            usize::MAX,
            Ok(None),
            8,
            Err(CancellationObservationError::Unavailable),
        ),
        (
            0,
            Err(CancellationObservationError::InvalidSignal),
            1,
            Err(CancellationObservationError::InvalidSignal),
        ),
        (
            1,
            Err(CancellationObservationError::ScopeMismatch),
            2,
            Err(CancellationObservationError::ScopeMismatch),
        ),
    ] {
        let observation = Observation {
            calls: AtomicUsize::new(0),
            unavailable,
            terminal,
        };
        assert_eq!(observe_command_cancellation(&observation), expected);
        assert_eq!(observation.calls.load(Ordering::Relaxed), expected_calls);
    }
}

const NOW: u64 = 1000;
const KEY: &[u8] = b"synthetic-command-state-secret-not-a-real-credential";
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "agentmage-command-owner-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        for name in ["state", "owned", "owned/worktree"] {
            fs::create_dir(path.join(name)).unwrap();
            fs::set_permissions(path.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(path.join("owned/worktree/sentinel"), b"preserve").unwrap();
        Self(path)
    }
    fn open(&self) -> LinuxCodingRepository {
        LinuxCodingRepository::open(&self.0.join("state"), KEY, NOW).unwrap()
    }
    fn synthetic_created(&self, owner: &mut LinuxCodingRepository) {
        // No native creation claim: inject only process-local provenance in tests.
        let path = self.0.join("owned/worktree");
        let original = self.0.join("original");
        fs::create_dir_all(&original).unwrap();
        if !original.join("sentinel").exists() {
            fs::write(original.join("sentinel"), b"preserve").unwrap();
        }
        owner.created = Some(CreatedWorktree {
            original: held_root(&original).unwrap(),
            original_path: original,
            edits: BTreeMap::new(),
            held: held_root(&path).unwrap(),
            path,
            owned: held_root(&self.0.join("owned")).unwrap(),
            creation_receipt_sha256: hash(b"synthetic creation"),
        });
    }
    fn pending(&self, owner: &mut LinuxCodingRepository) -> String {
        self.synthetic_created(owner);
        let registry = command_registry(spec()).unwrap();
        let pending = owner.command_preview(&registry, NOW).unwrap();
        let digest = pending.hash.clone();
        owner.command = Some(pending);
        digest
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn spec() -> CommandSpec {
    spec_with_bounds(CommandBounds::new(1000, 1024, 1024, 128 * 1024 * 1024, 8, 100).unwrap())
}
fn spec_with_bounds(bounds: CommandBounds) -> CommandSpec {
    CommandSpec::seal(
        "test-status",
        "1.0.0",
        "/usr/bin/true",
        hash(&fs::read("/usr/bin/true").unwrap()),
        vec![],
        CommandWorkingDirectory::OwnedWorktree,
        Default::default(),
        CommandRisk::Moderate,
        bounds,
    )
    .unwrap()
}
struct Synthetic {
    calls: Arc<AtomicUsize>,
    exit: i32,
    crash: bool,
    corrupt: bool,
}
impl BoundedCommandExecutor for Synthetic {
    type WorkingDirectory = LinuxAuthorizedWorkspace;
    fn execute(
        &mut self,
        permit: CommandLaunchPermit<'_>,
        _: &LinuxAuthorizedWorkspace,
        _: &dyn EffectCancellationObservation,
    ) -> std::result::Result<CommandPlatformResult, CommandExecutionFailure> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        assert_eq!(permit.command().spec_sha256, spec().spec_sha256);
        if self.crash {
            panic!("synthetic loss after durable consumption");
        }
        let stdout = b"untrusted \x1b[31m output".to_vec();
        Ok(CommandPlatformResult {
            termination: CommandTermination::Exited,
            exit_code: Some(self.exit),
            signal: None,
            stdout_sha256: if self.corrupt {
                hash(b"wrong")
            } else {
                hash(&stdout)
            },
            stdout_total_bytes: stdout.len() as u64,
            stdout,
            stderr: vec![],
            stderr_sha256: hash(b""),
            stderr_total_bytes: 0,
            elapsed_ms: 1,
            resource_usage: None,
            descendants_terminated: true,
            platform_code: "test.synthetic.command".into(),
        })
    }
}
fn execute(
    owner: &mut LinuxCodingRepository,
    digest: &str,
    calls: &Arc<AtomicUsize>,
    exit: i32,
    crash: bool,
    corrupt: bool,
) -> Result<Value> {
    let pending = owner.take_command(digest, true, NOW).unwrap().unwrap();
    owner.run_command(
        pending,
        Synthetic {
            calls: calls.clone(),
            exit,
            crash,
            corrupt,
        },
        NOW,
    )
}

#[test]
fn command_cancellation_is_exact_single_use_and_prelaunch_cancellation_spends_authority() {
    let f = Fixture::new();
    let mut owner = f.open();
    let calls = Arc::new(AtomicUsize::new(0));
    let digest = f.pending(&mut owner);
    let control = owner.command_cancellation(&digest).unwrap();
    assert_eq!(
        owner.validation_cancellation(&digest).err().unwrap().code(),
        "code.validation.choice-required"
    );
    control.cancel().unwrap();
    let denied = owner
        .decide_command_with_cancellation(&digest, false, NOW, Some(&control))
        .unwrap();
    assert_eq!(denied["outcome"], "denied");
    assert!(
        owner.inspect().unwrap()["receipts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let digest = f.pending(&mut owner);
    let control = owner.command_cancellation(&digest).unwrap();
    control.cancel().unwrap();
    control.cancel().unwrap();
    let pending = owner.take_command(&digest, true, NOW).unwrap().unwrap();
    let result = owner
        .run_command_with_cancellation(
            pending,
            Synthetic {
                calls: calls.clone(),
                exit: 0,
                crash: false,
                corrupt: false,
            },
            NOW,
            Some(&control),
        )
        .unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(result["outcome"], "cancelled");
    assert_eq!(result["command_receipt"]["termination"], "cancelled");
    assert_eq!(result["receipt"]["outcome"], "cancelled");
    assert!(owner.decide_command(&digest, true, NOW).is_err());
    let old = control;
    let digest = f.pending(&mut owner);
    assert_eq!(
        owner
            .decide_command_with_cancellation(&digest, true, NOW, Some(&old))
            .unwrap_err()
            .code(),
        "code.command.cancellation-stale"
    );
    assert!(
        owner.command.is_some(),
        "foreign handle must not consume consent"
    );
    let result = execute(&mut owner, &digest, &calls, 0, false, false).unwrap();
    assert_eq!(result["outcome"], "succeeded");
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
        b"preserve"
    );
    assert_eq!(
        owner.inspect().unwrap()["receipts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

struct CancelDuringExecution {
    entered: std::sync::mpsc::Sender<()>,
    cleanup_known: bool,
    shutdown: bool,
}
impl BoundedCommandExecutor for CancelDuringExecution {
    type WorkingDirectory = LinuxAuthorizedWorkspace;
    fn execute(
        &mut self,
        _: CommandLaunchPermit<'_>,
        _: &LinuxAuthorizedWorkspace,
        cancellation: &dyn EffectCancellationObservation,
    ) -> std::result::Result<CommandPlatformResult, CommandExecutionFailure> {
        assert!(
            cancellation
                .observe_effect_cancellation()
                .unwrap()
                .is_none()
        );
        self.entered.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(signal) = cancellation.observe_effect_cancellation().unwrap() {
                assert_eq!(
                    signal.reason,
                    if self.shutdown {
                        agentmage_kernel_contracts::CancellationReason::Shutdown
                    } else {
                        agentmage_kernel_contracts::CancellationReason::UserRequested
                    }
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "cancellation was not forwarded to executor"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        if !self.cleanup_known {
            return Err(CommandExecutionFailure::CleanupUncertain);
        }
        let stdout = b"partial untrusted output".to_vec();
        Ok(CommandPlatformResult {
            termination: CommandTermination::Cancelled,
            exit_code: None,
            signal: None,
            stdout_sha256: hash(&stdout),
            stdout_total_bytes: stdout.len() as u64,
            stdout,
            stderr: vec![],
            stderr_sha256: hash(b""),
            stderr_total_bytes: 0,
            elapsed_ms: 1,
            resource_usage: None,
            descendants_terminated: true,
            platform_code: "test.synthetic.cancelled".into(),
        })
    }
}

#[test]
fn live_attempt_cancellation_reaches_executor_and_never_turns_uncertain_cleanup_into_completion() {
    for validation in [false, true] {
        for cleanup_known in [false, true] {
            let f = Fixture::new();
            let mut owner = f.open();
            let command_digest = f.pending(&mut owner);
            let digest = if validation {
                validation_pending(&mut owner, None).unwrap()
            } else {
                command_digest
            };
            let control = owner.command_cancellation(&digest).unwrap();
            let signal = control.clone();
            let (entered, receive) = std::sync::mpsc::channel();
            let cancel_worker = std::thread::spawn(move || {
                receive.recv_timeout(Duration::from_secs(2)).unwrap();
                if validation {
                    signal.shutdown().unwrap();
                } else {
                    signal.cancel().unwrap();
                }
                signal.cancel().unwrap();
            });
            let pending = owner.take_command(&digest, true, NOW).unwrap().unwrap();
            let result = owner.run_command_with_cancellation(
                pending,
                CancelDuringExecution {
                    entered,
                    cleanup_known,
                    shutdown: validation,
                },
                NOW,
                Some(&control),
            );
            cancel_worker.join().unwrap();
            if cleanup_known {
                let result = result.unwrap();
                assert_eq!(result["outcome"], "cancelled");
                assert_eq!(result["command_receipt"]["termination"], "cancelled");
                assert_eq!(
                    result["stdout"],
                    json!(b"partial untrusted output".to_vec())
                );
                if validation {
                    assert_eq!(result["validation"]["validation_passed"], false);
                    assert_eq!(result["validation"]["receipt"]["status"], "cancelled");
                }
            } else {
                assert_eq!(result.unwrap_err().code(), "command.cleanup.uncertain");
                assert_eq!(
                    owner.inspect().unwrap()["receipts"][0]["outcome"],
                    "uncertain"
                );
            }
            assert!(owner.command.is_none());
            assert!(owner.command_cancellation(&digest).is_err());
            assert!(owner.decide_command(&digest, true, NOW).is_err());
            if validation {
                assert_eq!(owner.inspect_validation().unwrap()["attempts"], 1);
                assert!(owner.repair_feedback().is_err());
                assert!(validation_pending(&mut owner, None).is_err());
            }
            assert_eq!(
                fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
                b"preserve"
            );
            let retained = owner.inspect().unwrap()["receipts"].clone();
            drop(owner);
            let reopened = f.open();
            assert_eq!(reopened.inspect().unwrap()["receipts"], retained);
            assert!(reopened.command_cancellation(&digest).is_err());
        }
    }
}
#[test]
fn exact_commands_record_failure_then_fresh_approved_success_without_test_claims() {
    let f = Fixture::new();
    let mut owner = f.open();
    let calls = Arc::new(AtomicUsize::new(0));
    let digest = f.pending(&mut owner);
    assert_eq!(
        owner
            .decide_command(&hash(b"foreign"), true, NOW)
            .unwrap_err()
            .code(),
        "code.command.stale"
    );
    assert!(owner.inspect_command().unwrap()["pending"].is_object());
    let failed = execute(&mut owner, &digest, &calls, 1, false, false).unwrap();
    assert_eq!(failed["outcome"], "failed");
    assert_eq!(failed["test_suite_verified"], false);
    assert_eq!(
        failed["stdout"],
        json!(b"untrusted \x1b[31m output".to_vec())
    );
    assert_eq!(
        owner.decide_command(&digest, true, NOW).unwrap_err().code(),
        "code.command.choice-required"
    );
    let next = f.pending(&mut owner);
    assert_ne!(next, digest);
    let passed = execute(&mut owner, &next, &calls, 0, false, false).unwrap();
    assert_eq!(passed["outcome"], "succeeded");
    assert_eq!(passed["test_suite_verified"], false);
    let retained = owner.inspect().unwrap()["receipts"].clone();
    assert_eq!(retained.as_array().unwrap().len(), 2);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    drop(owner);
    let mut reopened = f.open();
    assert_eq!(reopened.inspect().unwrap()["receipts"], retained);
    assert!(reopened.inspect_command().unwrap()["pending"].is_null());
    assert!(reopened.inspect_command().unwrap()["worktree_root_sha256"].is_null());
    assert_eq!(
        reopened.prepare_command(spec(), NOW).unwrap_err().code(),
        "code.command.worktree-required"
    );
    assert_eq!(
        fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
        b"preserve"
    );
}
#[test]
fn denial_expiry_and_missing_ownership_do_not_dispatch() {
    let f = Fixture::new();
    let mut owner = f.open();
    assert_eq!(
        owner.prepare_command(spec(), NOW).unwrap_err().code(),
        "code.command.worktree-required"
    );
    let digest = f.pending(&mut owner);
    assert_eq!(
        owner.decide_command(&digest, false, NOW).unwrap()["outcome"],
        "denied"
    );
    let digest = f.pending(&mut owner);
    assert_eq!(
        owner
            .decide_command(&digest, true, NOW + LIFETIME_MS)
            .unwrap_err()
            .code(),
        "code.command.expired"
    );
    let digest = f.pending(&mut owner);
    owner.command.as_mut().unwrap().deadline = Instant::now();
    assert_eq!(
        owner.decide_command(&digest, true, NOW).unwrap_err().code(),
        "code.command.expired"
    );
    assert!(
        owner.inspect().unwrap()["receipts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn drift_of_worktree_or_management_directory_refuses_before_consumption() {
    for management in [false, true] {
        let f = Fixture::new();
        let mut owner = f.open();
        let digest = f.pending(&mut owner);
        let target = f.0.join(if management {
            "owned"
        } else {
            "owned/worktree"
        });
        let old = f.0.join("old");
        fs::rename(&target, &old).unwrap();
        if management {
            fs::create_dir(&target).unwrap();
        } else {
            symlink(&old, &target).unwrap();
        }
        let pending = owner.take_command(&digest, true, NOW).unwrap().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        assert_eq!(
            owner
                .run_command(
                    pending,
                    Synthetic {
                        calls: calls.clone(),
                        exit: 0,
                        crash: false,
                        corrupt: false
                    },
                    NOW
                )
                .unwrap_err()
                .code(),
            "code.command.scope"
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert!(
            owner.inspect().unwrap()["receipts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}
#[test]
fn tampered_specs_and_out_of_scope_commands_are_refused() {
    let original = spec();
    for change in 0..9 {
        let mut s = original.clone();
        match change {
            0 => s.network = true,
            1 => s.inherit_environment = true,
            2 => s.executable = "/usr/bin/sh".into(),
            3 => s.working_directory = CommandWorkingDirectory::EmptyScratch,
            4 => s.bounds.timeout_ms = 120001,
            5 => s.bounds.memory_bytes = 5 * 1024 * 1024 * 1024,
            6 => s.bounds.task_count = 65,
            7 => s.bounds.cpu_percent = 401,
            _ => s.bounds.stdout_bytes = 65537,
        }
        assert!(command_registry(s).is_err());
    }
}
#[test]
fn valid_sealed_commands_enforce_the_narrower_composition_limits() {
    let limit =
        CommandBounds::new(120_000, 65_536, 65_536, 4 * 1024 * 1024 * 1024, 64, 400).unwrap();
    // All composition ceilings are inclusive. These are valid kernel inputs,
    // not stale digests that can fail before testing the narrower local limits.
    assert!(command_registry(spec_with_bounds(limit)).is_ok());
    for change in 0..3 {
        let mut bounds = limit;
        match change {
            0 => bounds.timeout_ms += 1,
            1 => bounds.stdout_bytes += 1,
            _ => bounds.stderr_bytes += 1,
        }
        let sealed = spec_with_bounds(bounds);
        sealed.verify().unwrap();
        CommandRegistry::build(vec![sealed.clone()]).unwrap();
        assert_eq!(
            command_registry(sealed).unwrap_err().code(),
            "code.command.bounds"
        );
    }
    let original = spec();
    let scratch = CommandSpec::seal(
        original.template_id,
        original.template_version,
        original.executable,
        original.executable_sha256,
        original.arguments,
        CommandWorkingDirectory::EmptyScratch,
        original.environment,
        original.risk,
        original.bounds,
    )
    .unwrap();
    CommandRegistry::build(vec![scratch.clone()]).unwrap();
    assert_eq!(
        command_registry(scratch).unwrap_err().code(),
        "code.command.bounds"
    );
}
#[test]
fn corrupted_output_is_not_promoted_to_command_success() {
    let f = Fixture::new();
    let mut owner = f.open();
    let digest = f.pending(&mut owner);
    let calls = Arc::new(AtomicUsize::new(0));
    assert!(execute(&mut owner, &digest, &calls, 0, false, true).is_err());
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        owner.decide_command(&digest, true, NOW).unwrap_err().code(),
        "code.command.choice-required"
    );
    assert_ne!(
        owner.inspect().unwrap()["receipts"][0]["outcome"],
        "succeeded"
    );
}
#[test]
fn interruption_recovers_as_uncertain_without_restoring_command_or_ownership() {
    let f = Fixture::new();
    let mut owner = f.open();
    let digest = f.pending(&mut owner);
    let calls = Arc::new(AtomicUsize::new(0));
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = execute(&mut owner, &digest, &calls, 0, true, false);
        }))
        .is_err()
    );
    drop(owner);
    let mut owner =
        LinuxCodingRepository::open(&f.0.join("state"), KEY, NOW + LIFETIME_MS).unwrap();
    assert_eq!(
        owner.inspect().unwrap()["receipts"][0]["outcome"],
        "uncertain"
    );
    assert_eq!(
        owner.decide_command(&digest, true, NOW).unwrap_err().code(),
        "code.command.choice-required"
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}
#[test]
fn native_command_manifest_retains_trust_refusal_without_a_launch() {
    let f = Fixture::new();
    let mut owner = f.open();
    f.synthetic_created(&mut owner);
    match owner.prepare_command(spec(), NOW) {
        Err(e) => assert_eq!(e.code(), "linux.command.manifest.invalid"),
        Ok(preview) => {
            // On an admitted host this measures the actual sandbox runner only;
            // worktree provenance above remains synthetic, not native creation.
            let result =
                owner.decide_command(preview["preview_sha256"].as_str().unwrap(), true, NOW);
            assert!(result.is_ok(), "native sandbox unavailable: {result:?}");
            assert_eq!(result.unwrap()["outcome"], "succeeded");
            return;
        }
    }
    assert!(owner.inspect_command().unwrap()["pending"].is_null());
    assert!(
        owner.inspect().unwrap()["receipts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    eprintln!("native command not measured: exact launcher/executable trust refusal retained");
}

// Validation outcomes below are synthetic, under the same real held roots and
// encrypted authority. They do not claim a native child ran project tests.
use agentmage_kernel_engine::validation_template::ValidationKind;
struct Envelope {
    calls: Arc<AtomicUsize>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit: i32,
    corrupt: bool,
}
impl BoundedCommandExecutor for Envelope {
    type WorkingDirectory = LinuxAuthorizedWorkspace;
    fn execute(
        &mut self,
        _: CommandLaunchPermit<'_>,
        _: &LinuxAuthorizedWorkspace,
        _: &dyn EffectCancellationObservation,
    ) -> std::result::Result<CommandPlatformResult, CommandExecutionFailure> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(CommandPlatformResult {
            termination: CommandTermination::Exited,
            exit_code: Some(self.exit),
            signal: None,
            stdout: self.stdout.clone(),
            stdout_sha256: hash(if self.corrupt { b"wrong" } else { &self.stdout }),
            stdout_total_bytes: self.stdout.len() as u64,
            stderr: self.stderr.clone(),
            stderr_sha256: hash(&self.stderr),
            stderr_total_bytes: self.stderr.len() as u64,
            elapsed_ms: 1,
            resource_usage: None,
            descendants_terminated: true,
            platform_code: "test.synthetic.envelope".into(),
        })
    }
}
fn envelope(status: &str, passed: u32, failed: u32, skipped: u32) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":1,"status":status,"passed":passed,"failed":failed,
        "skipped":skipped,"duration_ms":1,"failed_names":if failed > 0 {vec!["synthetic.test"]} else {vec![]},
        "artifact_ids":[],"retry_count":0,"initial_failure_sha256":null})).unwrap()
}
fn validation_pending(owner: &mut LinuxCodingRepository, prior: Option<&str>) -> Result<String> {
    let registry = command_registry(spec()).unwrap();
    let pending = owner.validation_preview(
        &registry,
        ValidationKind::Unit,
        2,
        Default::default(),
        prior,
        NOW,
    )?;
    let digest = pending.hash.clone();
    owner.command = Some(pending);
    Ok(digest)
}
fn validation_execute(
    owner: &mut LinuxCodingRepository,
    digest: &str,
    stdout: Vec<u8>,
    exit: i32,
    calls: &Arc<AtomicUsize>,
    corrupt: bool,
) -> Result<Value> {
    let pending = owner.take_command(digest, true, NOW).unwrap().unwrap();
    owner.run_command(
        pending,
        Envelope {
            calls: calls.clone(),
            stdout,
            stderr: vec![],
            exit,
            corrupt,
        },
        NOW,
    )
}
#[test]
fn validation_failure_feedback_then_exact_fresh_rerun_passes_without_task_grade() {
    let f = Fixture::new();
    let mut owner = f.open();
    f.synthetic_created(&mut owner);
    let calls = Arc::new(AtomicUsize::new(0));
    let first = validation_pending(&mut owner, None).unwrap();
    assert!(owner.inspect_command().unwrap()["pending"].is_null());
    let preview = owner.inspect_validation().unwrap()["pending"].clone();
    assert_eq!(preview["attempt"], 1);
    assert_eq!(preview["validation_template"]["minimum_test_count"], 2);
    assert_eq!(
        preview["validation_template"]["parser_sha256"],
        hash(include_bytes!(
            "../../../kernel/engine/src/validation_result.rs"
        ))
    );
    assert_eq!(
        owner.decide_command(&first, true, NOW).unwrap_err().code(),
        "code.command.validation-choice-required"
    );
    let failed = validation_execute(
        &mut owner,
        &first,
        envelope("assertion_failed", 1, 1, 0),
        1,
        &calls,
        false,
    )
    .unwrap();
    assert_eq!(
        failed["validation"]["receipt"]["status"],
        "assertion_failed"
    );
    assert_eq!(
        failed["validation"]["receipt"]["failed_names"],
        json!(["synthetic.test"])
    );
    assert_eq!(failed["validation"]["receipt"]["approval_sha256"], first);
    assert_eq!(failed["validation"]["validation_passed"], false);
    assert_eq!(
        failed["validation"]["receipt"]["failure_classification"],
        "unclassified"
    );
    let prior = failed["validation"]["receipt"]["receipt_sha256"]
        .as_str()
        .unwrap();
    assert_eq!(
        validation_pending(&mut owner, None).unwrap_err().code(),
        "code.validation.stale"
    );
    assert_eq!(
        validation_pending(&mut owner, Some(&hash(b"foreign")))
            .unwrap_err()
            .code(),
        "code.validation.stale"
    );
    let second = validation_pending(&mut owner, Some(prior)).unwrap();
    assert_ne!(first, second);
    let passed = validation_execute(
        &mut owner,
        &second,
        envelope("passed", 2, 0, 0),
        0,
        &calls,
        false,
    )
    .unwrap();
    assert_eq!(passed["validation"]["validation_passed"], true);
    assert_eq!(passed["validation"]["attempt"], 2);
    assert_eq!(passed["rerun_plan"]["prior_receipt_sha256"], prior);
    assert_eq!(passed["rerun_plan"]["approval_sha256"], second);
    assert_eq!(
        passed["rerun_plan"]["command_spec_sha256"],
        spec().spec_sha256
    );
    assert_eq!(passed["test_suite_verified"], false);
    assert_eq!(passed["validation"]["current_workspace_verified"], false);
    assert_eq!(passed["validation"]["automatic_retry"], false);
    let latest = passed["validation"]["receipt"]["receipt_sha256"]
        .as_str()
        .unwrap();
    assert_eq!(
        validation_pending(&mut owner, Some(latest))
            .unwrap_err()
            .code(),
        "code.validation.rerun-forbidden"
    );
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    let receipts = owner.inspect().unwrap()["receipts"].clone();
    drop(owner);
    let reopened = f.open();
    let state = reopened.inspect_validation().unwrap();
    assert!(state["latest"].is_null());
    assert!(state["pending"].is_null());
    assert_eq!(state["attempts"], 0);
    assert_eq!(reopened.inspect().unwrap()["receipts"], receipts);
    assert!(reopened.created.is_none());
}

#[test]
fn repair_feedback_consumes_exact_failure_once_preserving_test_budget_and_write_consent() {
    let f = Fixture::new();
    let mut owner = f.open();
    assert!(owner.repair_feedback().is_err());
    f.synthetic_created(&mut owner);
    assert_eq!(
        owner.repair_feedback().unwrap_err().code(),
        "code.repair.failure-required"
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let first = validation_pending(&mut owner, None).unwrap();
    let failed = validation_execute(
        &mut owner,
        &first,
        envelope("assertion_failed", 1, 1, 0),
        1,
        &calls,
        false,
    )
    .unwrap();
    let feedback = owner.repair_feedback().unwrap();
    let prior = failed["validation"]["receipt"]["receipt_sha256"]
        .as_str()
        .unwrap();
    let digest = feedback["feedback_sha256"].as_str().unwrap();
    assert_eq!(feedback["receipt_sha256"], prior);
    assert_eq!(feedback["failed_names"], json!(["synthetic.test"]));
    assert_eq!(feedback["attempt"], 1);
    assert_eq!(
        feedback["stdout_sha256"],
        failed["validation"]["receipt"]["stdout_sha256"]
    );
    assert_eq!(feedback["output"]["stdout"]["truncated"], false);
    for (receipt, hash) in [("wrong", digest), (prior, "wrong")] {
        assert_eq!(
            owner
                .consume_repair_feedback(receipt, hash)
                .unwrap_err()
                .code(),
            "code.repair.stale"
        );
        assert_eq!(owner.repair_feedback().unwrap(), feedback);
    }
    assert_eq!(
        owner.consume_repair_feedback(prior, digest).unwrap(),
        feedback
    );
    assert_eq!(
        owner.repair_feedback().unwrap_err().code(),
        "code.repair.budget"
    );
    assert!(owner.consume_repair_feedback(prior, digest).is_err());
    let state = owner.inspect_validation().unwrap();
    assert_eq!(state["attempts"], 1);
    assert_eq!(state["repair_requested"], true);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert!(owner.inspect_edit().unwrap()["pending"].is_null());
    assert_eq!(
        fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
        b"preserve"
    );
    // The unchanged native exact write still needs independent consent. Editing
    // cannot reset the spent model request or the immutable test series.
    let edit = edit_preview(&mut owner);
    owner
        .decide_edit(edit["preview_sha256"].as_str().unwrap(), true, NOW)
        .unwrap();
    assert_eq!(
        owner.repair_feedback().unwrap_err().code(),
        "code.repair.budget"
    );
    let next = validation_pending(&mut owner, Some(prior)).unwrap();
    let second = validation_execute(
        &mut owner,
        &next,
        envelope("assertion_failed", 1, 1, 0),
        1,
        &calls,
        false,
    )
    .unwrap();
    let second_feedback = owner.repair_feedback().unwrap();
    assert_eq!(
        second_feedback["receipt_sha256"],
        second["validation"]["receipt"]["receipt_sha256"]
    );
    assert_ne!(second_feedback["receipt_sha256"], prior);
    let prior = second_feedback["receipt_sha256"].as_str().unwrap();
    owner
        .consume_repair_feedback(prior, second_feedback["feedback_sha256"].as_str().unwrap())
        .unwrap();
    let third = validation_pending(&mut owner, Some(prior)).unwrap();
    validation_execute(
        &mut owner,
        &third,
        envelope("assertion_failed", 1, 1, 0),
        1,
        &calls,
        false,
    )
    .unwrap();
    assert_eq!(
        owner.repair_feedback().unwrap_err().code(),
        "code.repair.budget"
    );
    assert_eq!(calls.load(Ordering::Relaxed), 3);
}

#[test]
fn repair_refuses_pass_uncertainty_restart_and_held_root_replacement() {
    for case in ["pass", "corrupt", "root", "restart"] {
        let f = Fixture::new();
        let mut owner = f.open();
        f.synthetic_created(&mut owner);
        let calls = Arc::new(AtomicUsize::new(0));
        let first = validation_pending(&mut owner, None).unwrap();
        let pass = case == "pass";
        let result = validation_execute(
            &mut owner,
            &first,
            envelope(
                if pass { "passed" } else { "assertion_failed" },
                if pass { 2 } else { 1 },
                if pass { 0 } else { 1 },
                0,
            ),
            if pass { 0 } else { 1 },
            &calls,
            case == "corrupt",
        );
        if case == "corrupt" {
            assert!(result.is_err());
        } else {
            result.unwrap();
        }
        match case {
            "root" => {
                let root = f.0.join("owned/worktree");
                fs::rename(&root, f.0.join("old-worktree")).unwrap();
                fs::create_dir(&root).unwrap();
            }
            "restart" => {
                drop(owner);
                owner = f.open();
            }
            _ => (),
        }
        assert!(owner.repair_feedback().is_err(), "{case}");
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
#[test]
fn validation_three_attempt_budget_survives_denial_and_cannot_reset_definition() {
    let f = Fixture::new();
    let mut owner = f.open();
    f.synthetic_created(&mut owner);
    let calls = Arc::new(AtomicUsize::new(0));
    let first = validation_pending(&mut owner, None).unwrap();
    assert_eq!(
        owner.decide_validation(&first, false, NOW).unwrap()["outcome"],
        "denied"
    );
    assert_eq!(owner.inspect_validation().unwrap()["attempts"], 0);
    let first = validation_pending(&mut owner, None).unwrap();
    assert_eq!(
        owner
            .decide_validation(&first, true, NOW + LIFETIME_MS)
            .unwrap_err()
            .code(),
        "code.command.expired"
    );
    assert_eq!(owner.inspect_validation().unwrap()["attempts"], 0);
    let mut prior = None;
    for attempt in 1..=3 {
        let digest = validation_pending(&mut owner, prior.as_deref()).unwrap();
        let value = validation_execute(
            &mut owner,
            &digest,
            envelope("assertion_failed", 1, 1, 0),
            1,
            &calls,
            false,
        )
        .unwrap();
        assert_eq!(value["validation"]["attempt"], attempt);
        prior = value["validation"]["receipt"]["receipt_sha256"]
            .as_str()
            .map(str::to_owned);
        if attempt == 1 {
            let registry = command_registry(spec()).unwrap();
            assert_eq!(
                owner
                    .validation_preview(
                        &registry,
                        ValidationKind::Unit,
                        1,
                        Default::default(),
                        prior.as_deref(),
                        NOW
                    )
                    .err()
                    .unwrap()
                    .code(),
                "code.validation.stale"
            );
        }
    }
    assert_eq!(
        validation_pending(&mut owner, prior.as_deref())
            .unwrap_err()
            .code(),
        "code.validation.budget"
    );
    assert_eq!(calls.load(Ordering::Relaxed), 3);
    assert!(owner.inspect_validation().unwrap()["pending"].is_null());
}
#[test]
fn validation_exit_zero_never_substitutes_for_strict_positive_test_counts() {
    for (stdout, exit, status) in [
        (b"all tests pass".to_vec(), 0, "malformed"),
        (envelope("passed", 0, 0, 0), 0, "zero_tests"),
        (envelope("passed", 1, 0, 0), 0, "zero_tests"),
        (envelope("passed", 2, 0, 0), 1, "malformed"),
        (envelope("skipped_only", 0, 0, 2), 0, "skipped_only"),
        (envelope("compile_failed", 0, 0, 0), 1, "compile_failed"),
    ] {
        let f = Fixture::new();
        let mut owner = f.open();
        f.synthetic_created(&mut owner);
        let calls = Arc::new(AtomicUsize::new(0));
        let digest = validation_pending(&mut owner, None).unwrap();
        let value = validation_execute(&mut owner, &digest, stdout, exit, &calls, false).unwrap();
        assert_eq!(value["validation"]["receipt"]["status"], status);
        assert_eq!(value["validation"]["validation_passed"], false);
        let prior = value["validation"]["receipt"]["receipt_sha256"]
            .as_str()
            .unwrap();
        if status == "compile_failed" {
            // Compilation is still unsuccessful with zero tests. Only an
            // inert, separately approvable retry preview becomes eligible.
            validation_pending(&mut owner, Some(prior)).unwrap();
            assert_eq!(owner.inspect_validation().unwrap()["attempts"], 1);
        } else {
            assert_eq!(
                validation_pending(&mut owner, Some(prior))
                    .unwrap_err()
                    .code(),
                "code.validation.rerun-forbidden"
            );
        }
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
#[test]
fn validation_corrupt_output_or_unwind_poison_the_series_before_retry() {
    for crash in [false, true] {
        let f = Fixture::new();
        let mut owner = f.open();
        f.synthetic_created(&mut owner);
        let calls = Arc::new(AtomicUsize::new(0));
        let digest = validation_pending(&mut owner, None).unwrap();
        if crash {
            let pending = owner.take_command(&digest, true, NOW).unwrap().unwrap();
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| owner.run_command(
                    pending,
                    Synthetic {
                        calls: calls.clone(),
                        exit: 0,
                        crash: true,
                        corrupt: false
                    },
                    NOW
                )))
                .is_err()
            );
        } else {
            assert!(
                validation_execute(
                    &mut owner,
                    &digest,
                    envelope("passed", 2, 0, 0),
                    0,
                    &calls,
                    true
                )
                .is_err()
            );
        }
        assert_eq!(owner.inspect_validation().unwrap()["attempts"], 1);
        assert!(owner.inspect_validation().unwrap()["latest"].is_null());
        assert_eq!(
            validation_pending(&mut owner, None).unwrap_err().code(),
            "code.validation.uncertain"
        );
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }
}
#[test]
fn validation_definition_scope_and_cross_route_checks_fail_closed() {
    let f = Fixture::new();
    let mut owner = f.open();
    let registry = command_registry(spec()).unwrap();
    assert_eq!(
        validation_pending(&mut owner, None).unwrap_err().code(),
        "code.command.worktree-required"
    );
    f.synthetic_created(&mut owner);
    assert_eq!(
        owner
            .validation_preview(
                &registry,
                ValidationKind::Build,
                2,
                Default::default(),
                None,
                NOW
            )
            .err()
            .unwrap()
            .code(),
        "code.validation.definition"
    );
    assert_eq!(
        owner
            .validation_preview(
                &registry,
                ValidationKind::Unit,
                0,
                Default::default(),
                None,
                NOW
            )
            .err()
            .unwrap()
            .code(),
        "code.validation.definition"
    );
    let command = f.pending(&mut owner);
    assert_eq!(
        owner
            .decide_validation(&command, true, NOW)
            .unwrap_err()
            .code(),
        "code.validation.choice-required"
    );
    assert_eq!(
        owner.inspect_command().unwrap()["pending"]["preview_sha256"],
        command
    );
    let digest = validation_pending(&mut owner, None).unwrap();
    assert_eq!(
        owner
            .decide_validation(&hash(b"foreign"), true, NOW)
            .unwrap_err()
            .code(),
        "code.command.stale"
    );
    let root = f.0.join("owned/worktree");
    fs::rename(&root, f.0.join("old")).unwrap();
    fs::create_dir(&root).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    assert_eq!(
        validation_execute(
            &mut owner,
            &digest,
            envelope("passed", 2, 0, 0),
            0,
            &calls,
            false
        )
        .unwrap_err()
        .code(),
        "code.command.scope"
    );
    assert_eq!(owner.inspect_validation().unwrap()["attempts"], 0);
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn native_validation_keeps_manifest_refusal_or_reports_empty_output_as_malformed() {
    let f = Fixture::new();
    let mut owner = f.open();
    f.synthetic_created(&mut owner);
    match owner.prepare_validation(
        spec(),
        ValidationKind::Unit,
        2,
        Default::default(),
        None,
        NOW,
    ) {
        Err(e) => {
            assert_eq!(e.code(), "linux.command.manifest.invalid");
            assert!(owner.inspect_validation().unwrap()["pending"].is_null());
            assert_eq!(owner.inspect_validation().unwrap()["attempts"], 0);
            assert!(
                owner.inspect().unwrap()["receipts"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            eprintln!(
                "native validation not measured: exact launcher/executable trust refusal retained"
            );
        }
        Ok(preview) => {
            // This branch uses the real native runner but synthetic creation
            // provenance. It is not an end-to-end created-worktree demonstration.
            let value = owner
                .decide_validation(preview["preview_sha256"].as_str().unwrap(), true, NOW)
                .unwrap();
            assert_eq!(value["outcome"], "succeeded");
            assert_eq!(value["validation"]["receipt"]["status"], "malformed");
            assert_eq!(value["validation"]["validation_passed"], false);
        }
    }
}

#[test]
fn owned_edit_writes_real_bytes_only_after_exact_approval_and_preserves_original() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("original")).unwrap();
    fs::write(f.0.join("original/sentinel"), b"preserve").unwrap();
    let mut owner = f.open();
    f.synthetic_created(&mut owner);
    let command_digest = f.pending(&mut owner);
    let scope = owner.select_created_sources(&["sentinel".into()]).unwrap();
    let preview = owner
        .prepare_edit(
            scope,
            &[crate::LinuxCodingReplacement {
                path: "sentinel",
                before_sha256: &hash(b"preserve"),
                content: "repaired\n",
            }],
            "Repair sentinel",
            NOW,
        )
        .unwrap();
    assert_eq!(
        fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
        b"preserve"
    );
    assert_eq!(preview["operation"], "edit-worktree");
    assert_eq!(
        preview["creation_receipt_sha256"],
        hash(b"synthetic creation")
    );
    let mut body = preview.clone();
    body.as_object_mut().unwrap().remove("preview_sha256");
    assert_eq!(
        preview["preview_sha256"],
        hash(&serde_json::to_vec(&body).unwrap())
    );
    assert!(preview["write_preview"]["preview_sha256"].is_string());
    assert_eq!(
        owner
            .decide_edit(&hash(b"wrong"), true, NOW)
            .unwrap_err()
            .code(),
        "code.edit.stale"
    );
    assert_eq!(owner.inspect_edit().unwrap()["pending"], preview);
    let digest = preview["preview_sha256"].as_str().unwrap();
    assert_eq!(
        owner.decide_command(digest, true, NOW).unwrap_err().code(),
        "code.command.stale"
    );
    assert_eq!(
        owner
            .decide_edit(&command_digest, true, NOW)
            .unwrap_err()
            .code(),
        "code.edit.stale"
    );
    assert_eq!(owner.inspect_edit().unwrap()["pending"], preview);
    let result = owner.decide_edit(digest, true, NOW).unwrap();
    assert_eq!(result["outcome"], "committed");
    assert_eq!(result["ownership_scope"], "service-created-worktree");
    assert_eq!(result["tested"], false);
    assert_eq!(result["write_result"]["isolation_verified"], false);
    assert!(
        result["write_result"]["receipts"]
            .as_array()
            .is_some_and(|r| !r.is_empty())
    );
    assert_eq!(owner.inspect_edit().unwrap()["latest"], result);
    assert_eq!(
        fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
        b"repaired\n"
    );
    assert_eq!(
        fs::read(f.0.join("original/sentinel")).unwrap(),
        b"preserve"
    );
    assert_eq!(
        owner.decide_edit(digest, true, NOW).unwrap_err().code(),
        "code.edit.choice-required"
    );
    assert_eq!(
        owner
            .decide_command(&command_digest, true, NOW)
            .unwrap_err()
            .code(),
        "code.command.choice-required"
    );
    // Writes reuse the existing process-only writer, not canonical durability.
    assert!(
        owner.inspect().unwrap()["receipts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(owner);
    let mut reopened = f.open();
    assert!(reopened.inspect_edit().unwrap()["pending"].is_null());
    assert!(reopened.inspect_edit().unwrap()["latest"].is_null());
    assert!(reopened.inspect_edit().unwrap()["worktree_root_sha256"].is_null());
    assert_eq!(
        reopened
            .select_created_sources(&["sentinel".into()])
            .err()
            .unwrap()
            .code(),
        "code.command.worktree-required"
    );
    assert_eq!(
        reopened.decide_edit(digest, true, NOW).unwrap_err().code(),
        "code.edit.choice-required"
    );
}

fn edit_preview(owner: &mut LinuxCodingRepository) -> Value {
    let scope = owner.select_created_sources(&["sentinel".into()]).unwrap();
    owner
        .prepare_edit(
            scope,
            &[crate::LinuxCodingReplacement {
                path: "sentinel",
                before_sha256: &hash(b"preserve"),
                content: "repaired\n",
            }],
            "Repair sentinel",
            NOW,
        )
        .unwrap()
}

#[test]
fn compilation_diagnostics_reach_one_repair_then_fresh_edit_and_exact_rerun() {
    use super::super::validation::LinuxCodingTestFormat;
    for format in [
        LinuxCodingTestFormat::RustLibtestTextV1,
        LinuxCodingTestFormat::AgentMageJsonV1,
    ] {
        let f = Fixture::new();
        let mut owner = f.open();
        f.synthetic_created(&mut owner);
        let calls = Arc::new(AtomicUsize::new(0));
        let registry = command_registry(spec()).unwrap();
        let pending = owner
            .validation_preview(&registry, ValidationKind::Unit, 1, format, None, NOW)
            .unwrap();
        let approval = pending.hash.clone();
        owner.command = Some(pending);
        let pending = owner.take_command(&approval, true, NOW).unwrap().unwrap();
        let text = matches!(format, LinuxCodingTestFormat::RustLibtestTextV1);
        let failed = owner.run_command(pending, Envelope {
            calls: calls.clone(),
            stdout: if text { vec![] } else { envelope("compile_failed", 0, 0, 0) },
            stderr: b"error[E0308]: mismatched types\nerror: could not compile `calc` (lib) due to 1 previous error\n".to_vec(),
            exit: 101, corrupt: false,
        }, NOW).unwrap();
        let receipt = &failed["validation"]["receipt"];
        assert_eq!(receipt["status"], "compile_failed");
        assert_eq!(receipt["passed"], 0);
        assert_eq!(receipt["failed"], 0);
        assert_eq!(receipt["failed_names"], json!([]));
        assert_eq!(failed["validation"]["validation_passed"], false);
        let prior = receipt["receipt_sha256"].as_str().unwrap();
        let feedback = owner.repair_feedback().unwrap();
        assert_eq!(feedback["status"], "compile_failed");
        assert_eq!(feedback["failed_names"], json!([]));
        assert_eq!(feedback["stderr_sha256"], receipt["stderr_sha256"]);
        assert!(
            feedback["output"]["stderr"]["head"]
                .as_str()
                .unwrap()
                .contains("error[E0308]")
        );
        owner
            .consume_repair_feedback(prior, feedback["feedback_sha256"].as_str().unwrap())
            .unwrap();
        assert!(
            owner
                .consume_repair_feedback(prior, feedback["feedback_sha256"].as_str().unwrap())
                .is_err()
        );
        assert_eq!(owner.inspect_validation().unwrap()["attempts"], 1);
        assert!(owner.inspect_edit().unwrap()["pending"].is_null());
        // Failure disclosure is not permission to edit. A declined diff writes
        // nothing; only a separately prepared and approved diff changes bytes.
        let edit = edit_preview(&mut owner);
        owner
            .decide_edit(edit["preview_sha256"].as_str().unwrap(), false, NOW)
            .unwrap();
        assert_eq!(
            fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
            b"preserve"
        );
        let edit = edit_preview(&mut owner);
        owner
            .decide_edit(edit["preview_sha256"].as_str().unwrap(), true, NOW)
            .unwrap();
        assert_eq!(
            fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
            b"repaired\n"
        );
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(
            owner
                .validation_preview(&registry, ValidationKind::Unit, 1, format, None, NOW)
                .is_err()
        );
        let pending = owner
            .validation_preview(&registry, ValidationKind::Unit, 1, format, Some(prior), NOW)
            .unwrap();
        let approval = pending.hash.clone();
        owner.command = Some(pending);
        let pending = owner.take_command(&approval, true, NOW).unwrap().unwrap();
        let passed = owner.run_command(pending, Envelope {
            calls: calls.clone(),
            stdout: if text { b"running 1 test\ntest add ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n".to_vec() } else { envelope("passed", 1, 0, 0) },
            stderr: vec![], exit: 0, corrupt: false,
        }, NOW).unwrap();
        assert_eq!(passed["validation"]["validation_passed"], true);
        assert_eq!(passed["validation"]["attempt"], 2);
        assert_eq!(passed["rerun_plan"]["prior_receipt_sha256"], prior);
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert!(owner.repair_feedback().is_err());
        assert_eq!(passed["test_suite_verified"], false);
    }
}

#[test]
fn owned_edits_reject_adoption_foreign_scopes_reserved_paths_and_changed_creation() {
    let f = Fixture::new();
    let mut owner = f.open();
    assert_eq!(
        owner
            .select_created_sources(&["sentinel".into()])
            .err()
            .unwrap()
            .code(),
        "code.command.worktree-required"
    );
    f.synthetic_created(&mut owner);
    for paths in [
        vec!["../state/authority".into()],
        vec![".git/config".into()],
        vec!["sentinel".into(), "sentinel".into()],
    ] {
        assert!(owner.select_created_sources(&paths).is_err());
    }
    fs::write(f.0.join("outside"), b"outside").unwrap();
    let foreign = LinuxCodingReadScope::select(&f.0, &["outside".into()]).unwrap();
    assert_eq!(
        owner
            .prepare_edit(
                foreign,
                &[crate::LinuxCodingReplacement {
                    path: "outside",
                    before_sha256: &hash(b"outside"),
                    content: "wrong",
                }],
                "Refuse",
                NOW
            )
            .unwrap_err()
            .code(),
        "code.edit.scope"
    );
    assert_eq!(fs::read(f.0.join("outside")).unwrap(), b"outside");
    let preview = edit_preview(&mut owner);
    owner.created.as_mut().unwrap().creation_receipt_sha256 = hash(b"different creation");
    assert_eq!(
        owner
            .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .unwrap_err()
            .code(),
        "code.edit.stale"
    );
    assert!(owner.inspect_edit().unwrap()["pending"].is_null());
    assert_eq!(
        fs::read(f.0.join("owned/worktree/sentinel")).unwrap(),
        b"preserve"
    );
}

#[test]
fn owned_edit_denial_expiry_source_drift_and_root_replacement_preserve_files() {
    for case in [
        "deny",
        "expire",
        "clock",
        "source",
        "root",
        "management",
        "symlink",
    ] {
        let f = Fixture::new();
        let mut owner = f.open();
        f.synthetic_created(&mut owner);
        let preview = edit_preview(&mut owner);
        let digest = preview["preview_sha256"].as_str().unwrap();
        let original = f.0.join("owned/worktree/sentinel");
        match case {
            "source" => fs::write(&original, b"human edit").unwrap(),
            "root" => {
                fs::rename(f.0.join("owned/worktree"), f.0.join("saved-worktree")).unwrap();
                fs::create_dir(f.0.join("owned/worktree")).unwrap();
                fs::write(&original, b"replacement").unwrap();
            }
            "management" => {
                fs::rename(f.0.join("owned"), f.0.join("saved-owned")).unwrap();
                fs::create_dir_all(f.0.join("owned/worktree")).unwrap();
                fs::write(&original, b"replacement").unwrap();
            }
            "symlink" => {
                fs::write(f.0.join("outside"), b"outside").unwrap();
                fs::remove_file(&original).unwrap();
                symlink(f.0.join("outside"), &original).unwrap();
            }
            _ => (),
        }
        let result = owner.decide_edit(
            digest,
            case != "deny",
            if case == "expire" {
                NOW + LIFETIME_MS
            } else if case == "clock" {
                NOW - 1
            } else {
                NOW
            },
        );
        match case {
            "deny" => assert_eq!(result.unwrap()["outcome"], "denied"),
            "source" | "symlink" => {
                assert_eq!(result.unwrap_err().code(), "code.write.transaction")
            }
            _ => assert!(result.is_err()),
        }
        let expected: &[u8] = match case {
            "source" => b"human edit",
            "root" | "management" => b"replacement",
            "symlink" => b"outside",
            _ => b"preserve",
        };
        assert_eq!(fs::read(&original).unwrap(), expected, "{case}");
        assert!(owner.inspect_edit().unwrap()["pending"].is_null());
        assert_eq!(
            owner.decide_edit(digest, true, NOW).unwrap_err().code(),
            "code.edit.choice-required"
        );
    }
}

#[test]
fn invalid_edit_preparation_keeps_the_existing_exact_preview_and_command() {
    let f = Fixture::new();
    let mut owner = f.open();
    let command = f.pending(&mut owner);
    let preview = edit_preview(&mut owner);
    for (path, digest) in [
        ("unselected", hash(b"preserve")),
        ("sentinel", hash(b"stale")),
    ] {
        let scope = owner.select_created_sources(&["sentinel".into()]).unwrap();
        assert!(
            owner
                .prepare_edit(
                    scope,
                    &[crate::LinuxCodingReplacement {
                        path,
                        before_sha256: &digest,
                        content: "wrong",
                    }],
                    "Refuse",
                    NOW
                )
                .is_err()
        );
        assert_eq!(owner.inspect_edit().unwrap()["pending"], preview);
        assert_eq!(
            owner.inspect_command().unwrap()["pending"]["preview_sha256"],
            command
        );
    }
    owner
        .decide_edit(preview["preview_sha256"].as_str().unwrap(), false, NOW)
        .unwrap();
    assert_eq!(
        owner.inspect_command().unwrap()["pending"]["preview_sha256"],
        command
    );
}

#[test]
fn owned_edit_invalidates_pending_validation_but_preserves_spent_budget_and_prior() {
    let f = Fixture::new();
    let mut owner = f.open();
    f.synthetic_created(&mut owner);
    let calls = Arc::new(AtomicUsize::new(0));
    let first = validation_pending(&mut owner, None).unwrap();
    let failed = validation_execute(
        &mut owner,
        &first,
        envelope("assertion_failed", 1, 1, 0),
        1,
        &calls,
        false,
    )
    .unwrap();
    let prior = failed["validation"]["receipt"]["receipt_sha256"]
        .as_str()
        .unwrap();
    let old = validation_pending(&mut owner, Some(prior)).unwrap();
    let validation_before = owner.inspect_validation().unwrap();
    let preview = edit_preview(&mut owner);
    assert_eq!(
        owner
            .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, NOW)
            .unwrap()["outcome"],
        "committed"
    );
    let validation_after = owner.inspect_validation().unwrap();
    assert!(validation_after["pending"].is_null());
    assert_eq!(validation_before["latest"], validation_after["latest"]);
    assert_eq!(validation_before["attempts"], validation_after["attempts"]);
    assert_eq!(validation_after["attempts"], 1);
    assert_eq!(
        owner.decide_validation(&old, true, NOW).unwrap_err().code(),
        "code.validation.choice-required"
    );
    assert_eq!(
        validation_pending(&mut owner, None).unwrap_err().code(),
        "code.validation.stale"
    );
    let fresh = validation_pending(&mut owner, Some(prior)).unwrap();
    assert_ne!(fresh, old);
    let passed = validation_execute(
        &mut owner,
        &fresh,
        envelope("passed", 2, 0, 0),
        0,
        &calls,
        false,
    )
    .unwrap();
    assert_eq!(passed["validation"]["attempt"], 2);
    assert_eq!(passed["validation"]["validation_passed"], true);
    assert_eq!(passed["validation"]["current_workspace_verified"], false);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn selected_project_output_formats_bind_the_preview_and_keep_exact_retry_budget() {
    use super::super::validation::LinuxCodingTestFormat;
    for (format, parser, fail, pass) in [
        (
            LinuxCodingTestFormat::RustLibtestTextV1,
            "rust_libtest_text_v1",
            "running 1 test\ntest add ... FAILED\n\nfailures:\nassertion failed\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n",
            "running 1 test\ntest add ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n",
        ),
        (
            LinuxCodingTestFormat::PythonUnittestTextV1,
            "python_unittest_text_v1",
            "test_add (calc.Tests.test_add) ... FAIL\n\n======================================================================\nFAIL: test_add (calc.Tests.test_add)\n----------------------------------------------------------------------\nAssertionError\n\n----------------------------------------------------------------------\nRan 1 test in 0.001s\n\nFAILED (failures=1)\n",
            "test_add (calc.Tests.test_add) ... ok\n\n----------------------------------------------------------------------\nRan 1 test in 0.001s\n\nOK\n",
        ),
    ] {
        let f = Fixture::new();
        let mut owner = f.open();
        f.synthetic_created(&mut owner);
        let calls = Arc::new(AtomicUsize::new(0));
        let registry = command_registry(spec()).unwrap();
        let pending = owner
            .validation_preview(&registry, ValidationKind::Unit, 1, format, None, NOW)
            .unwrap();
        assert_eq!(pending.preview["validation_template"]["parser"], parser);
        assert_eq!(
            pending.preview["validation_template"]["parser_sha256"],
            hash(
                &serde_json::to_vec(&[
                    hash(include_bytes!(
                        "../../../kernel/engine/src/validation_result.rs"
                    )),
                    hash(include_bytes!(
                        "../../../kernel/engine/src/validation_result_text.rs"
                    ))
                ])
                .unwrap()
            )
        );
        let mut prior = None;
        for attempt in 1..=2 {
            let pending = owner
                .validation_preview(
                    &registry,
                    ValidationKind::Unit,
                    1,
                    format,
                    prior.as_deref(),
                    NOW,
                )
                .unwrap();
            let approval = pending.hash.clone();
            owner.command = Some(pending);
            let pending = owner.take_command(&approval, true, NOW).unwrap().unwrap();
            let output = if attempt == 1 { fail } else { pass }.as_bytes().to_vec();
            let (stdout, stderr) = if parser == "python_unittest_text_v1" {
                (vec![], output)
            } else {
                (output, vec![])
            };
            let value = owner
                .run_command(
                    pending,
                    Envelope {
                        calls: calls.clone(),
                        stdout,
                        stderr,
                        exit: if attempt == 1 { 1 } else { 0 },
                        corrupt: false,
                    },
                    NOW,
                )
                .unwrap();
            assert_eq!(value["validation"]["attempt"], attempt);
            assert_eq!(value["validation"]["receipt"]["parser"], parser);
            assert_eq!(value["validation"]["receipt"]["approval_sha256"], approval);
            assert_eq!(value["validation"]["validation_passed"], attempt == 2);
            assert_eq!(value["test_suite_verified"], false);
            assert_eq!(value["validation"]["model_retry_connected"], false);
            if attempt == 1 {
                assert_eq!(value["validation"]["receipt"]["status"], "assertion_failed");
                let feedback = owner.repair_feedback().unwrap();
                assert_eq!(
                    feedback["receipt_sha256"],
                    value["validation"]["receipt"]["receipt_sha256"]
                );
                let channel = if parser == "python_unittest_text_v1" {
                    "stderr"
                } else {
                    "stdout"
                };
                assert!(
                    feedback["output"][channel]["head"]
                        .as_str()
                        .unwrap()
                        .contains("FAILED")
                );
                owner
                    .consume_repair_feedback(
                        feedback["receipt_sha256"].as_str().unwrap(),
                        feedback["feedback_sha256"].as_str().unwrap(),
                    )
                    .unwrap();
                prior = value["validation"]["receipt"]["receipt_sha256"]
                    .as_str()
                    .map(str::to_owned);
                // Parser selection cannot erase a failed result or reset budget.
                assert_eq!(
                    owner
                        .validation_preview(
                            &registry,
                            ValidationKind::Unit,
                            1,
                            LinuxCodingTestFormat::AgentMageJsonV1,
                            prior.as_deref(),
                            NOW
                        )
                        .err()
                        .unwrap()
                        .code(),
                    "code.validation.stale"
                );
            } else {
                assert_eq!(value["rerun_plan"]["prior_receipt_sha256"], json!(prior));
            }
        }
        assert_eq!(owner.inspect_validation().unwrap()["attempts"], 2);
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }
}
