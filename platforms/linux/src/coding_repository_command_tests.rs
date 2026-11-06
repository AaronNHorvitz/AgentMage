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
