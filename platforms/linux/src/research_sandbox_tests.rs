// Synthetic native syscall fixtures, not a research worker or network qualification.

use super::*;
use agentmage_kernel_engine::research_budget::{
    ResearchDepth, ResearchLimits, ResearchNetworkMode, ResearchScope,
};
use agentmage_kernel_engine::research_fetch::{PreparedPublicGet, PublicGetDraft, PublicGetTarget};
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::time::{SystemTime, UNIX_EPOCH};

fn packet() -> PreparedPublicGet {
    packet_at(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    )
}

fn packet_at(now: u64) -> PreparedPublicGet {
    let scope = ResearchScope::new(
        "research-probe".into(),
        ResearchDepth::Quick,
        ResearchNetworkMode::Ask,
        ResearchLimits::ceiling(ResearchDepth::Quick),
        BTreeSet::from(["docs.example.com".into()]),
        &["synthetic public query".into()],
    )
    .unwrap();
    PreparedPublicGet::prepare(
        &scope,
        PublicGetDraft {
            schema_version: 1,
            operation_id: "research-probe-1".into(),
            target: PublicGetTarget {
                domain: "docs.example.com".into(),
                path: "/".into(),
                query: vec![],
            },
            maximum_response_bytes: 1024,
            redirect_limit: 0,
            timeout_ms: 4000,
        },
        now,
        now,
    )
    .unwrap()
}

#[test]
fn resolver_service_delegation_is_data_only_and_rejects_untrusted_metadata() {
    let valid = (990, 0o040755, 990, 0o100644, 128, 1000);
    let check = |(directory_uid, directory_mode, file_uid, file_mode, size, caller_uid)| {
        valid_delegated_resolver(
            directory_uid,
            directory_mode,
            file_uid,
            file_mode,
            size,
            caller_uid,
        )
    };
    assert!(check(valid));
    for invalid in [
        (1000, valid.1, 1000, valid.3, valid.4, valid.5),
        (65534, valid.1, 65534, valid.3, valid.4, valid.5),
        (valid.0, valid.1, 1001, valid.3, valid.4, valid.5),
        (valid.0, 0o040775, valid.2, valid.3, valid.4, valid.5),
        (valid.0, 0o040757, valid.2, valid.3, valid.4, valid.5),
        (valid.0, 0o120755, valid.2, valid.3, valid.4, valid.5),
        (valid.0, valid.1, valid.2, 0o100664, valid.4, valid.5),
        (valid.0, valid.1, valid.2, 0o100646, valid.4, valid.5),
        (valid.0, valid.1, valid.2, 0o120644, valid.4, valid.5),
        (valid.0, valid.1, valid.2, 0o010644, valid.4, valid.5),
        (valid.0, valid.1, valid.2, valid.3, 0, valid.5),
        (valid.0, valid.1, valid.2, valid.3, 16385, valid.5),
    ] {
        assert!(!check(invalid), "{invalid:?}");
    }
}

#[test]
fn copied_development_worker_cannot_exceed_its_unchanged_native_memory_cap() {
    let limit = 128 * 1024 * 1024;
    assert!(!copied_worker_fits(149_193_840, limit)); // retained native OOM case
    assert!(!copied_worker_fits(limit as i64, limit));
    assert!(!copied_worker_fits(0, limit));
    assert!(!copied_worker_fits(-1, limit));
    assert!(copied_worker_fits(limit as i64 - 1, limit)); // lower bound only
    assert!(copied_worker_fits(512, limit));
}

#[test]
fn research_failure_keeps_observed_reason_distinct_from_uncertain_disclosure() {
    for outcome in [
        OperationOutcome::Failed,
        OperationOutcome::Denied,
        OperationOutcome::Cancelled,
        OperationOutcome::TimedOut,
        OperationOutcome::Uncertain,
    ] {
        for uncertain in [false, true] {
            let failure = LinuxPublicResearchFailure {
                outcome,
                reason: "synthetic-boundary-refusal",
                external_disclosure_uncertain: uncertain,
            };
            let effect = failure.effect();
            assert_eq!(failure.outcome(), outcome);
            assert_eq!(failure.external_disclosure_uncertain(), uncertain);
            assert_eq!(
                effect.outcome(),
                if uncertain {
                    OperationOutcome::Uncertain
                } else {
                    outcome
                }
            );
            assert_eq!(
                effect.state_change(),
                if uncertain {
                    StateChange::Uncertain
                } else {
                    StateChange::NotChanged
                }
            );
            assert_eq!(effect.result_sha256().len(), 64);
            let different = LinuxPublicResearchFailure {
                external_disclosure_uncertain: !uncertain,
                ..failure
            };
            assert_ne!(effect.result_sha256(), different.effect().result_sha256());
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn syscall_probe(nr: u32, args: [u32; 3]) -> Vec<u8> {
    // Fresh static ELF64 with a single tested syscall and no runtime libraries.
    // P = nonnegative result, D = exact EPERM, E = another kernel error. A socket
    // success closes its descriptor. No connect/send/listen, pathname, shell,
    // input parser or external server is present in the executable.
    fn imm(code: &mut Vec<u8>, op: u8, value: u32) {
        code.push(op);
        code.extend_from_slice(&value.to_le_bytes());
    }
    fn branch(code: &mut Vec<u8>, op: u8) -> usize {
        code.extend([op, 0]);
        code.len() - 1
    }
    fn bind(code: &mut [u8], from: usize, to: usize) {
        code[from] = i8::try_from(to as isize - (from + 1) as isize).unwrap() as u8;
    }
    let mut code = Vec::new();
    imm(&mut code, 0xbf, args[0]); // edi
    imm(&mut code, 0xbe, args[1]); // esi
    imm(&mut code, 0xba, args[2]); // edx
    imm(&mut code, 0xb8, nr); // eax
    code.extend([0x0f, 0x05]); // syscall
    imm(&mut code, 0xbb, 0); // ebx = P offset; syscall preserves rbx
    code.extend([0x48, 0x83, 0xf8, 0xff]); // cmp rax, -EPERM
    let denied = branch(&mut code, 0x74); // je
    code.extend([0x48, 0x85, 0xc0]); // test rax,rax
    let failed = branch(&mut code, 0x78); // js
    if nr == 41 {
        code.extend([0x89, 0xc7]); // mov edi,eax: returned socket
        imm(&mut code, 0xb8, 3); // close
        code.extend([0x0f, 0x05]);
    }
    let passed = branch(&mut code, 0xeb);
    let deny_label = code.len();
    imm(&mut code, 0xbb, 1); // D offset
    let deny_done = branch(&mut code, 0xeb);
    let fail_label = code.len();
    imm(&mut code, 0xbb, 2); // E offset
    let write_label = code.len();
    code.extend([0x48, 0x8d, 0x35]); // lea rsi, [rip+markers]
    let relative = code.len();
    code.extend([0; 4]);
    code.extend([0x48, 0x01, 0xde]); // add rsi,rbx
    imm(&mut code, 0xbf, 1); // stdout
    imm(&mut code, 0xba, 1); // one byte
    imm(&mut code, 0xb8, 1); // write
    code.extend([0x0f, 0x05]);
    imm(&mut code, 0xb8, 60); // exit
    code.extend([0x31, 0xff, 0x0f, 0x05]);
    let markers = code.len();
    code.extend(b"PDE");
    code[relative..relative + 4]
        .copy_from_slice(&i32::try_from(markers - relative - 4).unwrap().to_le_bytes());
    for (from, to) in [
        (denied, deny_label),
        (failed, fail_label),
        (passed, write_label),
        (deny_done, write_label),
    ] {
        bind(&mut code, from, to);
    }
    static_elf(code)
}

#[cfg(target_arch = "x86_64")]
fn static_elf(code: Vec<u8>) -> Vec<u8> {
    const ENTRY: usize = 120;
    const BASE: u64 = 0x0040_0000;
    let length = (ENTRY + code.len()) as u64;
    let mut elf = vec![0; ENTRY];
    elf[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    elf[16..18].copy_from_slice(&2_u16.to_le_bytes());
    elf[18..20].copy_from_slice(&62_u16.to_le_bytes());
    elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
    elf[24..32].copy_from_slice(&(BASE + ENTRY as u64).to_le_bytes());
    elf[32..40].copy_from_slice(&64_u64.to_le_bytes());
    elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
    elf[54..56].copy_from_slice(&56_u16.to_le_bytes());
    elf[56..58].copy_from_slice(&1_u16.to_le_bytes());
    elf[64..68].copy_from_slice(&1_u32.to_le_bytes());
    elf[68..72].copy_from_slice(&5_u32.to_le_bytes()); // read/execute, not write
    elf[80..88].copy_from_slice(&BASE.to_le_bytes());
    elf[88..96].copy_from_slice(&BASE.to_le_bytes());
    elf[96..104].copy_from_slice(&length.to_le_bytes());
    elf[104..112].copy_from_slice(&length.to_le_bytes());
    elf[112..120].copy_from_slice(&4096_u64.to_le_bytes());
    elf.extend(code);
    elf
}

#[cfg(target_arch = "x86_64")]
fn path_probe(path: &str, flags: u32, expected_errno: u8) -> Vec<u8> {
    assert!(!path.contains('\0'));
    let mut elf = syscall_probe(2, [0, flags, 0]); // open
    let address = 0x0040_0000_u32 + elf.len() as u32;
    elf[121..125].copy_from_slice(&address.to_le_bytes());
    let comparison = elf
        .windows(4)
        .position(|v| v == [0x48, 0x83, 0xf8, 0xff])
        .unwrap();
    elf[comparison + 3] = 0_u8.wrapping_sub(expected_errno);
    elf.extend(path.as_bytes());
    elf.push(0);
    let length = elf.len() as u64;
    elf[96..104].copy_from_slice(&length.to_le_bytes());
    elf[104..112].copy_from_slice(&length.to_le_bytes());
    elf
}

#[cfg(target_arch = "x86_64")]
fn environment_probe() -> Vec<u8> {
    // Print the kernel-provided environment from the initial ELF stack, one
    // NUL-terminated entry at a time. No file, socket, dynamic runtime or argv data.
    let mut code = vec![
        0x48, 0x8b, 0x04, 0x24, // mov rax,[rsp] = argc
        0x4c, 0x8d, 0x64, 0xc4, 0x10, // lea r12,[rsp+rax*8+16] = envp
    ];
    let next = code.len();
    code.extend([0x49, 0x8b, 0x34, 0x24]); // mov rsi,[r12]
    code.extend([0x48, 0x85, 0xf6, 0x74, 0]); // test rsi,rsi; jz exit
    let finished = code.len() - 1;
    code.extend([0x31, 0xd2]); // xor edx,edx
    let count = code.len();
    code.extend([0xff, 0xc2]); // inc edx
    code.extend([0x80, 0x7c, 0x16, 0xff, 0x00]); // cmp byte [rsi+rdx-1],0
    code.extend([0x75, 0]); // jne count
    let count_jump = code.len() - 1;
    code.extend([0xbf, 1, 0, 0, 0, 0xb8, 1, 0, 0, 0, 0x0f, 0x05]); // write
    code.extend([0x49, 0x83, 0xc4, 8, 0xeb, 0]); // add r12,8; jmp next
    let next_jump = code.len() - 1;
    let exit = code.len();
    code.extend([0xb8, 60, 0, 0, 0, 0x31, 0xff, 0x0f, 0x05]);
    for (from, to) in [(finished, exit), (count_jump, count), (next_jump, next)] {
        code[from] = i8::try_from(to as isize - (from + 1) as isize).unwrap() as u8;
    }
    static_elf(code)
}

fn fixture_root(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "agentmage-research-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    root
}

fn fixture_runner(root: &Path, bytes: &[u8]) -> LinuxPublicResearchRunner {
    let executable = root.join(WORKER);
    std::fs::write(&executable, bytes).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let worker =
        development_worker_artifact_kind(&executable, DevelopmentWorkerKind::PublicResearch)
            .unwrap();
    let manifest =
        LinuxPublicResearchManifest::with_worker(worker, digest_bytes(bytes), &[]).unwrap();
    LinuxPublicResearchRunner::new(
        manifest,
        LinuxSandboxLimits::new(128 * 1024 * 1024, 8, 25, 4, 128 * 1024).unwrap(),
    )
    .unwrap()
}

#[test]
#[cfg(target_arch = "x86_64")]
fn research_probe_is_bounded_static_and_uses_exact_native_call_arguments() {
    let bytes = syscall_probe(41, [2, 1, 0]);
    assert!(bytes.len() < 512);
    assert_eq!(&bytes[..7], b"\x7fELF\x02\x01\x01");
    assert_eq!(&bytes[120..125], &[0xbf, 2, 0, 0, 0]);
    assert_eq!(&bytes[135..140], &[0xb8, 41, 0, 0, 0]);
    assert!(bytes.ends_with(b"PDE"));
    assert_ne!(bytes, syscall_probe(41, [1, 1, 0]));
    assert_ne!(bytes, syscall_probe(41 | 0x4000_0000, [2, 1, 0]));
    let path = path_probe("/input/request", 0, 2);
    assert!(path.ends_with(b"/input/request\0"));
    assert_eq!(
        u64::from_le_bytes(path[96..104].try_into().unwrap()),
        path.len() as u64
    );
    assert!(environment_probe().len() < 256);
}

#[test]
#[cfg(target_arch = "x86_64")]
#[ignore = "native filesystem/env/descriptor isolation; owned sealed diagnostic ELF, no traffic"]
fn installed_research_namespace_projects_only_exact_inputs_and_no_host_descriptors() {
    let root = fixture_root("projection");
    let sentinel = root.join("private-sentinel");
    std::fs::write(&sentinel, b"synthetic private bytes never projected").unwrap();
    let mut cases = vec![
        ("/input/request".to_owned(), 0, 2, b'P'),
        ("/etc/resolv.conf".to_owned(), 0, 2, b'P'),
        ("/etc/nsswitch.conf".to_owned(), 0, 2, b'P'),
        (WORKER_GUEST.to_owned(), 0, 2, b'P'),
        ("/input/request".to_owned(), 1, 30, b'D'), // read-only mount, exact EROFS
        ("/etc/resolv.conf".to_owned(), 1, 30, b'D'),
        ("/etc/passwd".to_owned(), 0, 2, b'D'),
        ("/etc/hosts".to_owned(), 0, 2, b'D'),
        ("/workspace".to_owned(), 0, 2, b'D'),
        (sentinel.to_str().unwrap().to_owned(), 0, 2, b'D'),
        (format!("/proc/1/root{}", sentinel.display()), 0, 2, b'D'),
    ];
    for fd in 3..=24 {
        cases.push((format!("/proc/self/fd/{fd}"), 0, 2, b'D'));
    }
    for (index, (path, flags, errno, expected)) in cases.iter().enumerate() {
        let runner = fixture_runner(&root, &path_probe(path, *flags, *errno));
        let cancellation = super::super::LinuxSandboxCancellation::default();
        eprintln!("research-projection-fixture case={index}");
        let result = runner
            .run(
                packet().packet(),
                Instant::now() + Duration::from_secs(4),
                &SandboxCancellation::Local(&cancellation),
            )
            .expect("known owned cleanup");
        assert_eq!(result.outcome.unwrap(), OperationOutcome::Succeeded);
        assert!(result.output_complete && result.status.unwrap().success());
        assert_eq!(
            result.stdout.retained,
            [*expected],
            "projection case={index}"
        );
    }
    let runner = fixture_runner(&root, &environment_probe());
    let cancellation = super::super::LinuxSandboxCancellation::default();
    let result = runner
        .run(
            packet().packet(),
            Instant::now() + Duration::from_secs(4),
            &SandboxCancellation::Local(&cancellation),
        )
        .expect("known owned cleanup");
    assert_eq!(result.outcome.unwrap(), OperationOutcome::Succeeded);
    assert!(result.output_complete && result.status.unwrap().success());
    let entries = result
        .stdout
        .retained
        .split(|b| *b == 0)
        .filter(|v| !v.is_empty())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        entries,
        BTreeSet::from([b"LANG=C".as_slice(), b"PWD=/input".as_slice()])
    );
    assert_eq!(
        std::fs::read(sentinel).unwrap(),
        b"synthetic private bytes never projected"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(target_arch = "x86_64")]
fn lifecycle_probe(flood: bool) -> Vec<u8> {
    // Write a readiness marker then pause forever, or bounded-supervisor output
    // flood. No child, network, file write or CPU spin in the paused fixture.
    let mut code = vec![
        0xbf, 1, 0, 0, 0, 0xbe, 0, 0, 0, 0, 0xba, 1, 0, 0, 0, 0xb8, 1, 0, 0, 0, 0x0f, 0x05,
    ];
    if flood {
        code.extend([0xeb, 0xe8]); // jump back to write (24 bytes)
    } else {
        code.extend([0xb8, 34, 0, 0, 0, 0x0f, 0x05, 0xeb, 0xf7]); // pause; retry EINTR
    }
    let address = 0x0040_0000_u32 + 120 + code.len() as u32;
    code[6..10].copy_from_slice(&address.to_le_bytes());
    code.push(b'P');
    static_elf(code)
}

#[test]
#[cfg(target_arch = "x86_64")]
#[ignore = "native owned cancellation/deadline/output limits; sealed diagnostic ELF, no traffic"]
fn installed_research_cancellation_deadline_and_output_failure_leave_lane_reusable() {
    let root = fixture_root("lifecycle");
    for case in ["cancel", "deadline", "output", "reuse"] {
        let code = if case == "reuse" {
            syscall_probe(39, [0; 3])
        } else {
            lifecycle_probe(case == "output")
        };
        let runner = fixture_runner(&root, &code);
        let cancellation = Arc::new(super::super::LinuxSandboxCancellation::default());
        let controller = if case == "cancel" {
            let cancellation = cancellation.clone();
            Some(std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(500));
                cancellation.cancel();
            }))
        } else {
            None
        };
        eprintln!("research-lifecycle-fixture case={case}");
        let started = Instant::now();
        let duration = if case == "deadline" {
            Duration::from_millis(500)
        } else {
            Duration::from_secs(4)
        };
        let result = runner.run(
            packet().packet(),
            started + duration,
            &SandboxCancellation::Local(&cancellation),
        );
        if let Some(controller) = controller {
            controller.join().unwrap();
        }
        let result = result.expect("known exact cleanup");
        assert!(started.elapsed() < Duration::from_secs(8));
        assert_eq!(
            result.stdout.retained.first(),
            Some(&b'P'),
            "worker actually ran before {case}"
        );
        assert!(
            result.stdout.retained.len() as u64
                <= PublicGetResponse::maximum_frame_bytes(packet().packet())
        );
        match case {
            "cancel" => assert_eq!(result.outcome.unwrap(), OperationOutcome::Cancelled),
            "deadline" => assert_eq!(result.outcome.unwrap(), OperationOutcome::TimedOut),
            "output" => assert_eq!(
                result.outcome.unwrap_err().kind(),
                LinuxSandboxErrorKind::OutputLimitExceeded
            ),
            "reuse" => {
                assert_eq!(result.outcome.unwrap(), OperationOutcome::Succeeded);
                assert!(result.output_complete && result.status.unwrap().success());
                assert_eq!(result.stdout.retained, b"P");
            }
            _ => unreachable!(),
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "actual built optional worker with expired input; native startup/closure only, no DNS/HTTP"]
fn actual_research_worker_starts_inside_native_profile_and_refuses_expired_packet() {
    // Test-only explicit artifact, independently built/stripped/pinned by the
    // native verification driver. No production environment override is added.
    let executable = std::path::PathBuf::from(
        std::env::var_os("AGENTMAGE_NATIVE_RESEARCH_TEST_WORKER")
            .expect("verification driver must supply the exact built artifact"),
    );
    let worker =
        development_worker_artifact_kind(&executable, DevelopmentWorkerKind::PublicResearch)
            .unwrap();
    let expected = worker.sha256;
    let runtime = ["libgcc_s.so.1", "libc.so.6", "ld-linux-x86-64.so.2"]
        .into_iter()
        .map(|name| {
            LinuxWorkerRuntimeFile::new(
                std::fs::canonicalize(Path::new("/usr/lib64").join(name)).unwrap(),
                Path::new("/lib64").join(name),
            )
        })
        .collect::<Vec<_>>();
    let manifest = LinuxPublicResearchManifest::with_worker(worker, expected, &runtime).unwrap();
    eprintln!(
        "research-actual-worker-expired-input sha256={}",
        hex_digest(&expected)
    );
    let runner = LinuxPublicResearchRunner::new(
        manifest,
        LinuxSandboxLimits::new(128 * 1024 * 1024, 8, 25, 4, 128 * 1024).unwrap(),
    )
    .unwrap();
    let cancellation = super::super::LinuxSandboxCancellation::default();
    // Deliberately bypass only parent packet freshness in this module-private
    // negative test to verify the independent worker rejects before DNS/HTTP.
    let result = runner
        .run(
            packet_at(1).packet(),
            Instant::now() + Duration::from_secs(4),
            &SandboxCancellation::Local(&cancellation),
        )
        .expect("known owned cleanup");
    assert_eq!(result.outcome.unwrap(), OperationOutcome::Failed);
    assert!(result.output_complete);
    assert_eq!(result.status.unwrap().code(), Some(5));
    assert!(result.stdout.retained.is_empty());
    assert_eq!(result.stderr.retained, b"research.worker.input-denied\n");
}

#[test]
#[cfg(target_arch = "x86_64")]
#[ignore = "native research confinement diagnostic; owned units, sealed ELF, no network traffic"]
fn installed_research_filter_denies_nonclient_sockets_and_lane_is_reusable() {
    let root = std::env::temp_dir().join(format!(
        "agentmage-research-syscall-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let executable = root.join(WORKER);
    let cases = [
        (39, [0, 0, 0], b'P'),               // baseline getpid
        (41, [2, 1, 0], b'P'),               // IPv4 TCP socket, no connection
        (41, [10, 2, 0], b'P'),              // IPv6 UDP socket, no datagram
        (41, [1, 1, 0], b'D'),               // Unix socket forbidden
        (41, [16, 3, 0], b'D'),              // netlink forbidden
        (41, [2, 3, 0], b'D'),               // raw IP forbidden
        (41, [2, 1 | 0x400, 0], b'D'),       // unknown flags forbidden
        (41, [2, 1, 255], b'D'),             // nonclient protocol forbidden
        (49, [0, 0, 0], b'D'),               // bind fails at syscall boundary, no actual address
        (50, [0, 0, 0], b'D'),               // listen
        (57, [0, 0, 0], b'D'),               // child creation
        (101, [0, 0, 0], b'D'),              // ptrace
        (39 | 0x4000_0000, [0, 0, 0], b'D'), // x32 alias
        (39, [0, 0, 0], b'P'),               // lane still usable after refusals
    ];
    for (nr, args, expected) in cases {
        let bytes = syscall_probe(nr, args);
        std::fs::write(&executable, &bytes).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let artifact =
            development_worker_artifact_kind(&executable, DevelopmentWorkerKind::PublicResearch)
                .unwrap();
        assert!(
            super::super::development_worker_artifact(&executable).is_err(),
            "read-only kind cannot admit research worker"
        );
        assert!(
            LinuxPublicResearchManifest::verify(&executable, digest_bytes(&bytes), &[]).is_err(),
            "development file is not production trust"
        );
        let manifest =
            LinuxPublicResearchManifest::with_worker(artifact, digest_bytes(&bytes), &[]).unwrap();
        let limits = LinuxSandboxLimits::new(128 * 1024 * 1024, 8, 25, 4, 128 * 1024).unwrap();
        let runner = LinuxPublicResearchRunner::new(manifest, limits).unwrap();
        let prepared = packet();
        let cancellation = super::super::LinuxSandboxCancellation::default();
        eprintln!(
            "research-syscall-fixture nr={nr} args={args:?} sha256={}",
            hex_digest(&digest_bytes(&bytes))
        );
        let result = runner
            .run(
                prepared.packet(),
                Instant::now() + Duration::from_secs(4),
                &SandboxCancellation::Local(&cancellation),
            )
            .expect("exact owned cleanup");
        assert_eq!(result.outcome.unwrap(), OperationOutcome::Succeeded);
        assert!(result.output_complete && result.status.unwrap().success());
        assert_eq!(
            result.stdout.retained,
            [expected],
            "actual installed filter; E is never a pass"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
