//! Separate client syscall profile of the existing native sandbox owner.
//! It does not change the offline policy or by itself activate research.
//! This filter constrains socket families/types/protocols, NOT peers or ports.
//! Exact public destination/DNS/peer/TLS mediation remains inside the independently
//! pinned trusted Rust worker under Decision0084. Native canaries are mandatory.

use rustix::net::{AddressFamily, SocketFlags, SocketType, ipproto};
use seccompiler::{TargetArch, compile_from_json};
use serde_json::{Value, json};

use super::{
    DENIED_SYSCALLS, LinuxSandboxError, LinuxSandboxErrorKind, error, serialize_native_bpf,
};

const CLIENT_SYSCALLS: &[&str] = &[
    "connect", "recvfrom", "recvmmsg", "recvmsg", "sendmmsg", "sendmsg", "sendto", "shutdown",
    "socket",
];

pub(super) const POLICY_ID: &str = "agentmage.linux.public-research.client.v1";

fn not_equal(index: u8, value: u64) -> Value {
    // Linux treats these socket arguments as native ints; x32/foreign ABI refusal
    // is separately retained by the parent's existing serialization guard.
    json!({"index": index, "type": "dword", "op": "ne", "val": value})
}

fn source() -> Value {
    // Only this separate connected profile selects the exact client calls. The
    // original deny set, offline compiler and all default workers remain unchanged.
    let mut rules = DENIED_SYSCALLS
        .iter()
        .filter(|syscall| !CLIENT_SYSCALLS.contains(syscall))
        .map(|syscall| json!({"syscall": syscall}))
        .collect::<Vec<_>>();
    // This fixed, single-threaded worker is not a general command runner. The
    // initial exec occurs after filter installation; no child creation is needed.
    rules.push(json!({"syscall": "clone"}));
    #[cfg(target_arch = "x86_64")]
    for syscall in ["fork", "vfork"] {
        rules.push(json!({"syscall": syscall}));
    }
    rules.push(json!({"syscall": "socket", "args": [
        not_equal(0, u64::from(AddressFamily::INET.as_raw())),
        not_equal(0, u64::from(AddressFamily::INET6.as_raw())),
    ]}));
    let permitted_types = [SocketType::STREAM, SocketType::DGRAM]
        .into_iter()
        .flat_map(|kind| {
            [
                SocketFlags::empty(),
                SocketFlags::NONBLOCK,
                SocketFlags::CLOEXEC,
                SocketFlags::NONBLOCK | SocketFlags::CLOEXEC,
            ]
            .into_iter()
            .map(move |flags| not_equal(1, u64::from(kind.as_raw() | flags.bits())))
        })
        .collect::<Vec<_>>();
    // One AND-bound denial rule excludes every value outside these exact eight.
    // Unknown flag bits are denied, not masked away into a permitted socket type.
    rules.push(json!({"syscall": "socket", "args": permitted_types}));
    rules.push(json!({"syscall": "socket", "args": [
        not_equal(2, 0),
        not_equal(2, u64::from(ipproto::TCP.as_raw().get())),
        not_equal(2, u64::from(ipproto::UDP.as_raw().get())),
    ]}));
    json!({"worker": {"mismatch_action": "allow", "match_action": {"errno": 1},
                      "filter": rules}})
}

pub(super) fn compile() -> Result<Vec<u8>, LinuxSandboxError> {
    let denied = || error(LinuxSandboxErrorKind::SeccompUnavailable);
    let architecture: TargetArch = std::env::consts::ARCH.try_into().map_err(|_| denied())?;
    let encoded = serde_json::to_vec(&source()).map_err(|_| denied())?;
    let mut filters = compile_from_json(encoded.as_slice(), architecture).map_err(|_| denied())?;
    let program = filters.remove("worker").ok_or_else(denied)?;
    let bytes = serialize_native_bpf(&program, architecture)?;
    // Preserve the same atomic policy-release maximum used by the shared owner.
    if bytes.is_empty() || bytes.len() > 4096 || !bytes.len().is_multiple_of(8) {
        return Err(denied());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALLOW: u32 = 0x7fff_0000;
    const EPERM: u32 = 0x0005_0001;
    const KILL: u32 = 0x8000_0000;

    // Test-only forward BPF evaluator, not a production execution engine or
    // installed-filter proof. Unknown opcodes/loads or escape fail the fixture.
    fn evaluate(bytes: &[u8], nr: u32, arch: u32, args: [u64; 3]) -> u32 {
        assert!(!bytes.is_empty() && bytes.len().is_multiple_of(8));
        let mut pc = 0_usize;
        let mut acc = 0_u32;
        for _ in 0..bytes.len() / 8 {
            let op = bytes.get(pc * 8..pc * 8 + 8).expect("bounded instruction");
            let code = u16::from_ne_bytes(op[..2].try_into().unwrap());
            let k = u32::from_ne_bytes(op[4..].try_into().unwrap());
            let branch = |yes| usize::from(if yes { op[2] } else { op[3] });
            let skip = match code {
                0x20 => {
                    acc = match k {
                        0 => nr,
                        4 => arch,
                        16 | 24 | 32 => args[((k - 16) / 8) as usize] as u32,
                        20 | 28 | 36 => (args[((k - 20) / 8) as usize] >> 32) as u32,
                        _ => panic!("unexpected load"),
                    };
                    0
                }
                0x05 => k as usize,
                0x15 => branch(acc == k),
                0x25 => branch(acc > k),
                0x35 => branch(acc >= k),
                0x06 => return k,
                _ => panic!("unexpected instruction"),
            };
            pc = pc.checked_add(skip + 1).unwrap();
        }
        panic!("missing bounded return");
    }

    #[test]
    fn connected_policy_is_distinct_and_stays_inside_atomic_release_ceiling() {
        let policy = compile().unwrap();
        assert!(policy.len() <= 4096);
        assert_ne!(policy, super::super::compile_seccomp_policy().unwrap());
        for syscall in CLIENT_SYSCALLS {
            assert!(
                DENIED_SYSCALLS.contains(syscall),
                "offline policy stays closed"
            );
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn compiled_socket_policy_admits_only_exact_client_argument_sets() {
        let policy = compile().unwrap();
        for family in [2_u64, 10] {
            for kind in [1_u64, 2] {
                for flags in [0_u64, 0x800, 0x80000, 0x80800] {
                    for protocol in [0_u64, 6, 17] {
                        assert_eq!(
                            evaluate(&policy, 41, 0xc000_003e, [family, kind | flags, protocol]),
                            ALLOW
                        );
                    }
                }
            }
        }
        for args in [
            [0, 1, 0],
            [1, 1, 0],
            [16, 1, 0],
            [17, 1, 0],
            [2, 3, 0],
            [2, 1 | 0x400, 0],
            [2, 1, 1],
            [2, 1, 255],
            [2, 1, u64::from(u32::MAX)],
            [u64::from(u32::MAX), 1, 0],
        ] {
            assert_eq!(evaluate(&policy, 41, 0xc000_003e, args), EPERM, "{args:?}");
        }
        // Linux consumes these socket arguments as native ints, not pointer data.
        assert_eq!(
            evaluate(&policy, 41, 0xc000_003e, [2 | (1 << 32), 1, 0]),
            ALLOW
        );
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn compiled_policy_keeps_child_server_privileged_and_alias_syscalls_denied() {
        let policy = compile().unwrap();
        for nr in [
            43, 49, 50, 53, 56, 57, 58, 101, 165, 272, 298, 308, 321, 435,
        ] {
            assert_eq!(evaluate(&policy, nr, 0xc000_003e, [0; 3]), EPERM, "{nr}");
        }
        for nr in [39, 41, 42] {
            assert_eq!(
                evaluate(&policy, nr | 0x4000_0000, 0xc000_003e, [2, 1, 0]),
                EPERM
            );
            assert_eq!(evaluate(&policy, nr, 0x4000_0003, [2, 1, 0]), KILL);
        }
        assert_eq!(evaluate(&policy, 39, 0xc000_003e, [0; 3]), ALLOW);
        assert_eq!(evaluate(&policy, 42, 0xc000_003e, [0; 3]), ALLOW);
    }
}
