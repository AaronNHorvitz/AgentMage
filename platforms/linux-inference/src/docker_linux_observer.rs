//! Bounded Linux observations used by the privileged Docker topology collector.

use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::AsFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use rustix::process::{Pid, PidfdFlags, pidfd_open};
use rustix::thread::{LinkNameSpaceType, move_into_link_name_space};
use sha2::{Digest, Sha256};

const MAX_EXECUTABLE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PROC_RECORD_BYTES: u64 = 1024 * 1024;
const TCP_LISTEN_STATE: &str = "0A";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinuxObserverError {
    TargetIdentity,
    ProcessRecord,
    ObjectIdentity,
    Namespace,
    NetworkRecord,
}

impl LinuxObserverError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::TargetIdentity => "docker-collector.target-identity",
            Self::ProcessRecord => "docker-collector.process-record",
            Self::ObjectIdentity => "docker-collector.object-identity",
            Self::Namespace => "docker-collector.namespace",
            Self::NetworkRecord => "docker-collector.network-record",
        }
    }
}

impl fmt::Display for LinuxObserverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for LinuxObserverError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProcessObservation {
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) groups: Vec<u32>,
    pub(crate) start_time_ticks: u64,
    pub(crate) executable_sha256: [u8; 32],
    pub(crate) cgroup_sha256: [u8; 32],
    pub(crate) network_namespace_sha256: [u8; 32],
    pub(crate) mount_namespace_sha256: [u8; 32],
    pub(crate) effective_capabilities: u64,
    pub(crate) no_new_privileges: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UnixSocketObservation {
    pub(crate) identity_sha256: [u8; 32],
    pub(crate) owner_uid: u32,
    pub(crate) group_gid: u32,
    pub(crate) mode: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirectoryObservation {
    pub(crate) owner_uid: u32,
    pub(crate) group_gid: u32,
    pub(crate) mode: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NetworkObservation {
    pub(crate) active_interface_count: u8,
    pub(crate) loopback_up: bool,
    pub(crate) non_loopback_interface_count: u8,
    pub(crate) non_local_route_count: u8,
    pub(crate) raw_listener_count: u8,
    pub(crate) raw_wildcard_v4_listener_count: u8,
    pub(crate) raw_wildcard_v6_listener_count: u8,
    pub(crate) non_loopback_listener_count: u8,
    pub(crate) management_listener_count: u8,
    pub(crate) outbound_bytes: u64,
}

pub(crate) fn observe_process(
    pid: i32,
    expected_start_time_ticks: u64,
) -> Result<ProcessObservation, LinuxObserverError> {
    let pid = Pid::from_raw(pid).ok_or(LinuxObserverError::TargetIdentity)?;
    if pid.as_raw_nonzero().get() <= 1 || expected_start_time_ticks == 0 {
        return Err(LinuxObserverError::TargetIdentity);
    }
    let _held =
        pidfd_open(pid, PidfdFlags::empty()).map_err(|_| LinuxObserverError::TargetIdentity)?;
    let root = PathBuf::from(format!("/proc/{}", pid.as_raw_nonzero().get()));
    let first_start = read_process_start(&root)?;
    if first_start != expected_start_time_ticks {
        return Err(LinuxObserverError::TargetIdentity);
    }
    let status = read_bounded(&root.join("status"), MAX_PROC_RECORD_BYTES)?;
    let status = parse_status(&status)?;
    let executable_link =
        fs::read_link(root.join("exe")).map_err(|_| LinuxObserverError::TargetIdentity)?;
    if executable_link
        .as_os_str()
        .as_bytes()
        .ends_with(b" (deleted)")
    {
        return Err(LinuxObserverError::TargetIdentity);
    }
    let executable_sha256 = hash_file_bounded(&root.join("exe"), MAX_EXECUTABLE_BYTES)?;
    let cgroup = read_bounded(&root.join("cgroup"), MAX_PROC_RECORD_BYTES)?;
    let network_namespace_sha256 = namespace_identity(&root.join("ns/net"))?;
    let mount_namespace_sha256 = namespace_identity(&root.join("ns/mnt"))?;
    if read_process_start(&root)? != first_start {
        return Err(LinuxObserverError::TargetIdentity);
    }
    Ok(ProcessObservation {
        uid: status.uid,
        gid: status.gid,
        groups: status.groups,
        start_time_ticks: first_start,
        executable_sha256,
        cgroup_sha256: Sha256::digest(cgroup).into(),
        network_namespace_sha256,
        mount_namespace_sha256,
        effective_capabilities: status.effective_capabilities,
        no_new_privileges: status.no_new_privileges,
    })
}

pub(crate) fn observe_current_process(pid: i32) -> Result<ProcessObservation, LinuxObserverError> {
    if pid <= 1 {
        return Err(LinuxObserverError::TargetIdentity);
    }
    let root = PathBuf::from(format!("/proc/{pid}"));
    let start = read_process_start(&root)?;
    observe_process(pid, start)
}

pub(crate) fn observe_unix_socket(
    path: &Path,
) -> Result<UnixSocketObservation, LinuxObserverError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| LinuxObserverError::ObjectIdentity)?;
    if !metadata.file_type().is_socket() {
        return Err(LinuxObserverError::ObjectIdentity);
    }
    Ok(UnixSocketObservation {
        identity_sha256: object_identity(&metadata),
        owner_uid: metadata.uid(),
        group_gid: metadata.gid(),
        mode: metadata.mode() & 0o777,
    })
}

pub(crate) fn observe_directory(path: &Path) -> Result<DirectoryObservation, LinuxObserverError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| LinuxObserverError::ObjectIdentity)?;
    if !metadata.file_type().is_dir() {
        return Err(LinuxObserverError::ObjectIdentity);
    }
    Ok(DirectoryObservation {
        owner_uid: metadata.uid(),
        group_gid: metadata.gid(),
        mode: metadata.mode() & 0o777,
    })
}

pub(crate) fn hash_regular_file(path: &Path, maximum: u64) -> Result<[u8; 32], LinuxObserverError> {
    hash_file_bounded(path, maximum)
}

pub(crate) fn count_processes_by_executable(
    executable_sha256: [u8; 32],
) -> Result<u8, LinuxObserverError> {
    if executable_sha256 == [0; 32] {
        return Err(LinuxObserverError::TargetIdentity);
    }
    let mut count = 0_u8;
    for entry in fs::read_dir("/proc").map_err(|_| LinuxObserverError::ProcessRecord)? {
        let entry = entry.map_err(|_| LinuxObserverError::ProcessRecord)?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<i32>().ok())
        else {
            continue;
        };
        let path = PathBuf::from(format!("/proc/{pid}/exe"));
        match hash_file_bounded(&path, MAX_EXECUTABLE_BYTES) {
            Ok(observed) if observed == executable_sha256 => {
                count = count
                    .checked_add(1)
                    .ok_or(LinuxObserverError::ProcessRecord)?;
            }
            Ok(_) | Err(LinuxObserverError::TargetIdentity) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(count)
}

pub(crate) fn observe_network_namespace(
    pid: i32,
    raw_port: u16,
) -> Result<NetworkObservation, LinuxObserverError> {
    let pid = Pid::from_raw(pid).ok_or(LinuxObserverError::TargetIdentity)?;
    let _held =
        pidfd_open(pid, PidfdFlags::empty()).map_err(|_| LinuxObserverError::TargetIdentity)?;
    let original = File::open("/proc/self/ns/net").map_err(|_| LinuxObserverError::Namespace)?;
    let target = File::open(format!("/proc/{}/ns/net", pid.as_raw_nonzero().get()))
        .map_err(|_| LinuxObserverError::Namespace)?;
    move_into_link_name_space(target.as_fd(), Some(LinkNameSpaceType::Network))
        .map_err(|_| LinuxObserverError::Namespace)?;
    let observed = observe_current_network(raw_port);
    let restored = move_into_link_name_space(original.as_fd(), Some(LinkNameSpaceType::Network));
    if restored.is_err() {
        return Err(LinuxObserverError::Namespace);
    }
    observed
}

pub(crate) fn observe_current_network(
    raw_port: u16,
) -> Result<NetworkObservation, LinuxObserverError> {
    let mut active = 0_u8;
    let mut non_loopback = 0_u8;
    let loopback_up = parse_ipv6_loopback(&read_bounded(
        Path::new("/proc/net/if_inet6"),
        MAX_PROC_RECORD_BYTES,
    )?)?;
    let mut outbound_bytes = 0_u64;
    let devices = parse_network_devices(&read_bounded(
        Path::new("/proc/net/dev"),
        MAX_PROC_RECORD_BYTES,
    )?)?;
    for (name, transmitted) in &devices {
        active = active
            .checked_add(1)
            .ok_or(LinuxObserverError::NetworkRecord)?;
        if name.as_slice() != b"lo" {
            non_loopback = non_loopback
                .checked_add(1)
                .ok_or(LinuxObserverError::NetworkRecord)?;
            outbound_bytes = outbound_bytes
                .checked_add(*transmitted)
                .ok_or(LinuxObserverError::NetworkRecord)?;
        }
    }
    let routes = parse_route_count(&read_bounded(
        Path::new("/proc/net/route"),
        MAX_PROC_RECORD_BYTES,
    )?)?;
    let v4 = parse_tcp_listeners(
        &read_bounded(Path::new("/proc/net/tcp"), MAX_PROC_RECORD_BYTES)?,
        raw_port,
        false,
    )?;
    let v6 = parse_tcp_listeners(
        &read_bounded(Path::new("/proc/net/tcp6"), MAX_PROC_RECORD_BYTES)?,
        raw_port,
        true,
    )?;
    Ok(NetworkObservation {
        active_interface_count: active,
        loopback_up,
        non_loopback_interface_count: non_loopback,
        non_local_route_count: routes,
        raw_listener_count: v4
            .raw
            .checked_add(v6.raw)
            .ok_or(LinuxObserverError::NetworkRecord)?,
        raw_wildcard_v4_listener_count: v4.wildcard,
        raw_wildcard_v6_listener_count: v6.wildcard,
        non_loopback_listener_count: v4
            .specific_non_loopback
            .checked_add(v6.specific_non_loopback)
            .ok_or(LinuxObserverError::NetworkRecord)?,
        management_listener_count: v4
            .management
            .checked_add(v6.management)
            .ok_or(LinuxObserverError::NetworkRecord)?,
        outbound_bytes,
    })
}

fn read_process_start(root: &Path) -> Result<u64, LinuxObserverError> {
    let stat = read_bounded(&root.join("stat"), MAX_PROC_RECORD_BYTES)?;
    let stat = std::str::from_utf8(&stat).map_err(|_| LinuxObserverError::ProcessRecord)?;
    let command_end = stat.rfind(')').ok_or(LinuxObserverError::ProcessRecord)?;
    stat.get(command_end + 2..)
        .and_then(|fields| fields.split_whitespace().nth(19))
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .ok_or(LinuxObserverError::ProcessRecord)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedStatus {
    uid: u32,
    gid: u32,
    groups: Vec<u32>,
    effective_capabilities: u64,
    no_new_privileges: bool,
}

fn parse_status(bytes: &[u8]) -> Result<ParsedStatus, LinuxObserverError> {
    let text = std::str::from_utf8(bytes).map_err(|_| LinuxObserverError::ProcessRecord)?;
    let uid = parse_equal_identity_line(text, "Uid:")?;
    let gid = parse_equal_identity_line(text, "Gid:")?;
    let groups = field(text, "Groups:")?
        .split_whitespace()
        .map(|value| value.parse().map_err(|_| LinuxObserverError::ProcessRecord))
        .collect::<Result<Vec<_>, _>>()?;
    let effective_capabilities = u64::from_str_radix(field(text, "CapEff:")?.trim(), 16)
        .map_err(|_| LinuxObserverError::ProcessRecord)?;
    let no_new_privileges = match field(text, "NoNewPrivs:")?.trim() {
        "0" => false,
        "1" => true,
        _ => return Err(LinuxObserverError::ProcessRecord),
    };
    Ok(ParsedStatus {
        uid,
        gid,
        groups,
        effective_capabilities,
        no_new_privileges,
    })
}

fn parse_equal_identity_line(text: &str, name: &str) -> Result<u32, LinuxObserverError> {
    let values = field(text, name)?
        .split_whitespace()
        .map(|value| value.parse().map_err(|_| LinuxObserverError::ProcessRecord))
        .collect::<Result<Vec<u32>, _>>()?;
    let first = *values.first().ok_or(LinuxObserverError::ProcessRecord)?;
    if values.len() != 4 || values.iter().any(|value| *value != first) {
        return Err(LinuxObserverError::ProcessRecord);
    }
    Ok(first)
}

fn field<'a>(text: &'a str, name: &str) -> Result<&'a str, LinuxObserverError> {
    text.lines()
        .find_map(|line| line.strip_prefix(name))
        .ok_or(LinuxObserverError::ProcessRecord)
}

fn namespace_identity(path: &Path) -> Result<[u8; 32], LinuxObserverError> {
    let file = File::open(path).map_err(|_| LinuxObserverError::ObjectIdentity)?;
    let metadata = file
        .metadata()
        .map_err(|_| LinuxObserverError::ObjectIdentity)?;
    Ok(object_identity(&metadata))
}

fn object_identity(metadata: &fs::Metadata) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(metadata.dev().to_be_bytes());
    digest.update(metadata.ino().to_be_bytes());
    digest.update(metadata.uid().to_be_bytes());
    digest.update(metadata.gid().to_be_bytes());
    digest.update(metadata.mode().to_be_bytes());
    digest.finalize().into()
}

fn hash_file_bounded(path: &Path, maximum: u64) -> Result<[u8; 32], LinuxObserverError> {
    let mut file = File::open(path).map_err(|_| LinuxObserverError::TargetIdentity)?;
    let metadata = file
        .metadata()
        .map_err(|_| LinuxObserverError::TargetIdentity)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > maximum {
        return Err(LinuxObserverError::TargetIdentity);
    }
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 32 * 1024];
    let mut total = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| LinuxObserverError::TargetIdentity)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or(LinuxObserverError::TargetIdentity)?;
        if total > maximum {
            return Err(LinuxObserverError::TargetIdentity);
        }
        digest.update(&buffer[..read]);
    }
    Ok(digest.finalize().into())
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, LinuxObserverError> {
    let mut file = File::open(path).map_err(|_| LinuxObserverError::ProcessRecord)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| LinuxObserverError::ProcessRecord)?;
    if bytes.is_empty() || bytes.len() as u64 > maximum {
        return Err(LinuxObserverError::ProcessRecord);
    }
    Ok(bytes)
}

fn parse_route_count(bytes: &[u8]) -> Result<u8, LinuxObserverError> {
    let text = std::str::from_utf8(bytes).map_err(|_| LinuxObserverError::NetworkRecord)?;
    text.lines().skip(1).try_fold(0_u8, |count, line| {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 8 {
            return Err(LinuxObserverError::NetworkRecord);
        }
        if fields[0] == "lo" {
            Ok(count)
        } else {
            count
                .checked_add(1)
                .ok_or(LinuxObserverError::NetworkRecord)
        }
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParsedListeners {
    raw: u8,
    wildcard: u8,
    specific_non_loopback: u8,
    management: u8,
}

fn parse_tcp_listeners(
    bytes: &[u8],
    raw_port: u16,
    ipv6: bool,
) -> Result<ParsedListeners, LinuxObserverError> {
    let text = std::str::from_utf8(bytes).map_err(|_| LinuxObserverError::NetworkRecord)?;
    let mut raw = 0_u8;
    let mut wildcard = 0_u8;
    let mut specific_non_loopback = 0_u8;
    let mut management = 0_u8;
    for line in text.lines().skip(1) {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 4 || fields[3] != TCP_LISTEN_STATE {
            continue;
        }
        let (address, port) = fields[1]
            .split_once(':')
            .ok_or(LinuxObserverError::NetworkRecord)?;
        let port = u16::from_str_radix(port, 16).map_err(|_| LinuxObserverError::NetworkRecord)?;
        if port != raw_port {
            management = management
                .checked_add(1)
                .ok_or(LinuxObserverError::NetworkRecord)?;
            continue;
        }
        raw = raw
            .checked_add(1)
            .ok_or(LinuxObserverError::NetworkRecord)?;
        let (wildcard_address, loopback) = if ipv6 {
            (
                address == "00000000000000000000000000000000",
                address == "00000000000000000000000001000000",
            )
        } else {
            (address == "00000000", address == "0100007F")
        };
        if wildcard_address {
            wildcard = wildcard
                .checked_add(1)
                .ok_or(LinuxObserverError::NetworkRecord)?;
        } else if !loopback {
            specific_non_loopback = specific_non_loopback
                .checked_add(1)
                .ok_or(LinuxObserverError::NetworkRecord)?;
        }
    }
    Ok(ParsedListeners {
        raw,
        wildcard,
        specific_non_loopback,
        management,
    })
}

fn parse_network_devices(
    bytes: &[u8],
) -> Result<std::collections::BTreeMap<Vec<u8>, u64>, LinuxObserverError> {
    let mut output = std::collections::BTreeMap::new();
    for line in bytes.split(|byte| *byte == b'\n').skip(2) {
        let Some(separator) = line.iter().position(|byte| *byte == b':') else {
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            return Err(LinuxObserverError::NetworkRecord);
        };
        let (name, counters_with_separator) = line.split_at(separator);
        let counters = &counters_with_separator[1..];
        let name = name
            .iter()
            .copied()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect::<Vec<_>>();
        let counters = std::str::from_utf8(counters)
            .map_err(|_| LinuxObserverError::NetworkRecord)?
            .split_whitespace()
            .collect::<Vec<_>>();
        if name.is_empty() || counters.len() != 16 {
            return Err(LinuxObserverError::NetworkRecord);
        }
        let transmitted = counters[8]
            .parse()
            .map_err(|_| LinuxObserverError::NetworkRecord)?;
        if output.insert(name, transmitted).is_some() {
            return Err(LinuxObserverError::NetworkRecord);
        }
    }
    Ok(output)
}

fn parse_ipv6_loopback(bytes: &[u8]) -> Result<bool, LinuxObserverError> {
    let text = std::str::from_utf8(bytes).map_err(|_| LinuxObserverError::NetworkRecord)?;
    let mut loopback = false;
    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 6
            || fields[0].len() != 32
            || !fields[0].bytes().all(|byte| byte.is_ascii_hexdigit())
            || u32::from_str_radix(fields[1], 16).is_err()
            || u8::from_str_radix(fields[2], 16).is_err()
            || u8::from_str_radix(fields[3], 16).is_err()
            || u8::from_str_radix(fields[4], 16).is_err()
            || fields[5].is_empty()
        {
            return Err(LinuxObserverError::NetworkRecord);
        }
        if fields[0] == "00000000000000000000000000000001"
            && fields[2] == "80"
            && fields[3] == "10"
            && fields[5] == "lo"
        {
            loopback = true;
        }
    }
    Ok(loopback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_is_held_and_revalidated() {
        let pid = i32::try_from(std::process::id()).expect("test PID");
        let root = PathBuf::from(format!("/proc/{pid}"));
        let start = read_process_start(&root).expect("current start time");
        let observed = observe_process(pid, start).expect("current process observation");
        assert_eq!(observed.start_time_ticks, start);
        assert_ne!(observed.executable_sha256, [0; 32]);
        assert_ne!(observed.cgroup_sha256, [0; 32]);
        assert_ne!(observed.network_namespace_sha256, [0; 32]);
        assert_ne!(observed.mount_namespace_sha256, [0; 32]);
        assert_eq!(
            observe_process(pid, start.saturating_add(1)),
            Err(LinuxObserverError::TargetIdentity)
        );
    }

    #[test]
    fn status_requires_complete_equal_identity_and_closed_security_fields() {
        let valid = b"Uid:\t1000\t1000\t1000\t1000\nGid:\t1001\t1001\t1001\t1001\nGroups:\t10 1001\nCapEff:\t0000000000000000\nNoNewPrivs:\t1\n";
        let parsed = parse_status(valid).expect("valid status");
        assert_eq!(parsed.uid, 1000);
        assert_eq!(parsed.gid, 1001);
        assert_eq!(parsed.groups, vec![10, 1001]);
        assert!(parsed.no_new_privileges);
        for invalid in [
            replace_bytes(valid, b"1000\t1000\t1000\t1000", b"1000\t0\t1000\t1000"),
            replace_bytes(valid, b"NoNewPrivs:\t1", b"NoNewPrivs:\t2"),
            replace_bytes(valid, b"CapEff:\t0000000000000000", b"CapEff:\tinvalid"),
        ] {
            assert_eq!(
                parse_status(&invalid),
                Err(LinuxObserverError::ProcessRecord)
            );
        }
    }

    #[test]
    fn tcp_route_and_device_parsers_are_exact() {
        let tcp = b"sl local_address rem_address st\n0: 00000000:3092 00000000:0000 0A\n1: 0100007F:3092 00000000:0000 0A\n2: 0100007F:0050 00000000:0000 0A\n";
        assert_eq!(
            parse_tcp_listeners(tcp, 12_434, false),
            Ok(ParsedListeners {
                raw: 2,
                wildcard: 1,
                specific_non_loopback: 0,
                management: 1,
            })
        );
        let tcp6 = b"sl local_address rem_address st\n0: 00000000000000000000000000000000:3092 00000000000000000000000000000000:0000 0A\n1: 00000000000000000000000001000000:3092 00000000000000000000000000000000:0000 0A\n";
        assert_eq!(
            parse_tcp_listeners(tcp6, 12_434, true),
            Ok(ParsedListeners {
                raw: 2,
                wildcard: 1,
                specific_non_loopback: 0,
                management: 0,
            })
        );
        let routes = b"Iface Destination Gateway Flags RefCnt Use Metric Mask MTU Window IRTT\nlo 0000007F 00000000 0001 0 0 0 000000FF 0 0 0\neth0 00000000 0100000A 0003 0 0 0 00000000 0 0 0\n";
        assert_eq!(parse_route_count(routes), Ok(1));
        let devices = b"Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n lo: 1 0 0 0 0 0 0 0 2 0 0 0 0 0 0 0\n eth0: 3 0 0 0 0 0 0 0 4 0 0 0 0 0 0 0\n";
        let parsed = parse_network_devices(devices).expect("network devices");
        assert_eq!(parsed.get(b"lo".as_slice()), Some(&2));
        assert_eq!(parsed.get(b"eth0".as_slice()), Some(&4));
        let inet6 = b"00000000000000000000000000000001 01 80 10 80 lo\nfe800000000000000000000000000001 02 40 20 80 eth0\n";
        assert_eq!(parse_ipv6_loopback(inet6), Ok(true));
        assert_eq!(
            parse_ipv6_loopback(b"fe800000000000000000000000000001 02 40 20 80 eth0\n"),
            Ok(false)
        );
        assert_eq!(
            parse_ipv6_loopback(b"invalid 01 80 10 80 lo\n"),
            Err(LinuxObserverError::NetworkRecord)
        );
    }

    fn replace_bytes(input: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
        let index = input
            .windows(old.len())
            .position(|window| window == old)
            .expect("test fixture contains replacement target");
        let mut output = Vec::with_capacity(input.len() - old.len() + new.len());
        output.extend_from_slice(&input[..index]);
        output.extend_from_slice(new);
        output.extend_from_slice(&input[index + old.len()..]);
        output
    }
}
