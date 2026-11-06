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
        owner.created = Some(CreatedWorktree {
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
            stderr: vec![],
            stderr_sha256: hash(b""),
            stderr_total_bytes: 0,
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
    let pending = owner.validation_preview(&registry, ValidationKind::Unit, 2, prior, NOW)?;
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
                    .validation_preview(&registry, ValidationKind::Unit, 1, prior.as_deref(), NOW)
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
        assert_eq!(
            validation_pending(&mut owner, Some(prior))
                .unwrap_err()
                .code(),
            "code.validation.rerun-forbidden"
        );
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
            .validation_preview(&registry, ValidationKind::Build, 2, None, NOW)
            .err()
            .unwrap()
            .code(),
        "code.validation.definition"
    );
    assert_eq!(
        owner
            .validation_preview(&registry, ValidationKind::Unit, 0, None, NOW)
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
    match owner.prepare_validation(spec(), ValidationKind::Unit, 2, None, NOW) {
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
