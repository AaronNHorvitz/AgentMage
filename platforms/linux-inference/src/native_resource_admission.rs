//! Measured prelaunch restrictions for the existing prepared 32K development lane.
//! This does not grant model admission, replace the campaign supervisor or reserve
//! another application's resources. No inference is launched by this module.

use std::fs::File;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use agentmage_kernel_contracts::ModelRuntimeFailure;
use rustix::fs::{Mode, OFlags, fcntl_getfl, fcntl_setfl, open};
use serde::Serialize;
use sha2::{Digest, Sha256};

const GIB: u64 = 1024 * 1024 * 1024;
const NVIDIA_SMI: &str = "/usr/bin/nvidia-smi";

/// Exact preparation policy, never a model admission or machine reservation.
#[derive(Clone, Debug)]
pub struct NativeDevelopmentResourcePolicy {
    manifest_sha256: String,
    nvidia_smi_sha256: String,
}

#[derive(Debug, Serialize)]
struct Observation {
    manifest_sha256: String,
    observed_epoch_ms: u64,
    available_ram_bytes: u64,
    gpu_used_mib: u64,
    gpu_free_mib: u64,
    gpu_total_mib: u64,
    memory_high_bytes: u64,
    memory_max_bytes: u64,
    memory_swap_max_bytes: u64,
    cpu_quota_us: u64,
    cpu_period_us: u64,
}

impl NativeDevelopmentResourcePolicy {
    /// Accepts only the existing bounded preparation profile. Changing any frozen
    /// launch/resource limit requires an explicit source/profile re-evaluation.
    pub fn from_preparation_manifest(bytes: &[u8]) -> Result<Self, ModelRuntimeFailure> {
        if bytes.len() > 64 * 1024 {
            return Err(failure("profile-invalid"));
        }
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| failure("profile-invalid"))?;
        for (field, expected) in [
            ("schema_version", 1),
            ("context_tokens", 32768),
            ("output_tokens", 4096),
            ("parallel_slots", 1),
            ("threads", 4),
            ("batch_tokens", 256),
            ("microbatch_tokens", 128),
            ("minimum_available_ram_gib", 16),
            ("minimum_free_vram_mib", 21000),
            ("maximum_total_vram_used_mib", 22528),
        ] {
            if value.get(field).and_then(serde_json::Value::as_u64) != Some(expected) {
                return Err(failure("profile-invalid"));
            }
        }
        if value["scope"] != "synthetic-development-preparation-only"
            || value["product_enabled"] != false
            || value["cache_type_k"] != "q8_0"
            || value["cache_type_v"] != "q8_0"
        {
            return Err(failure("profile-invalid"));
        }
        let pin = value
            .pointer("/platform_tuple/nvidia_smi_sha256")
            .and_then(serde_json::Value::as_str)
            .filter(|pin| {
                pin.len() == 64
                    && pin.bytes().any(|b| b != b'0')
                    && pin
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
            .ok_or_else(|| failure("profile-invalid"))?;
        Ok(Self {
            manifest_sha256: digest(bytes),
            nvidia_smi_sha256: pin.into(),
        })
    }

    /// Rejects an unbounded host before potentially large model/manifest reads.
    /// This performs no GPU query and cannot replace the leased launch check.
    pub(crate) fn verify_scope_before_manifest(&self) -> Result<(), ModelRuntimeFailure> {
        self.observe_system().map(|_| ())
    }

    fn observe_system(&self) -> Result<Observation, ModelRuntimeFailure> {
        let available_ram_bytes =
            available_ram(&bounded_text(Path::new("/proc/meminfo"), 64 * 1024)?)?;
        if available_ram_bytes < 16 * GIB {
            return Err(failure("ram-contended"));
        }
        let group = cgroup_path(&bounded_text(Path::new("/proc/self/cgroup"), 4096)?)?;
        let root = Path::new("/sys/fs/cgroup").join(group.trim_start_matches('/'));
        let read_control = |name: &str| bounded_text(&root.join(name), 128);
        let memory_high_bytes = integer(read_control("memory.high")?.trim())?;
        let memory_max_bytes = integer(read_control("memory.max")?.trim())?;
        let memory_swap_max_bytes = integer(read_control("memory.swap.max")?.trim())?;
        let (cpu_quota_us, cpu_period_us) = cpu_max(&read_control("cpu.max")?)?;
        let observed_epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|value| u64::try_from(value.as_millis()).ok())
            .filter(|value| *value > 0)
            .ok_or_else(|| failure("observation-invalid"))?;
        // Refuse an unbounded host before starting even the pinned GPU observer.
        validate_scope(
            memory_high_bytes,
            memory_max_bytes,
            memory_swap_max_bytes,
            cpu_quota_us,
            cpu_period_us,
        )?;
        Ok(Observation {
            manifest_sha256: self.manifest_sha256.clone(),
            observed_epoch_ms,
            available_ram_bytes,
            gpu_used_mib: 0,
            gpu_free_mib: 0,
            gpu_total_mib: 0,
            memory_high_bytes,
            memory_max_bytes,
            memory_swap_max_bytes,
            cpu_quota_us,
            cpu_period_us,
        })
    }

    /// Called only inside the native lease, immediately before inference launch.
    pub(crate) fn verify_before_launch(&self) -> Result<String, ModelRuntimeFailure> {
        let mut observation = self.observe_system()?;
        (
            observation.gpu_used_mib,
            observation.gpu_free_mib,
            observation.gpu_total_mib,
        ) = gpu_memory(&query_gpu(&self.nvidia_smi_sha256)?)?;
        // The bounded observer can take up to ten seconds. Recheck available RAM
        // after it exits instead of treating the earlier snapshot as a reservation.
        observation.available_ram_bytes =
            available_ram(&bounded_text(Path::new("/proc/meminfo"), 64 * 1024)?)?;
        validate_observation(&observation)?;
        serde_json::to_string(&observation).map_err(|_| failure("observation-invalid"))
    }
}

fn failure(suffix: &str) -> ModelRuntimeFailure {
    ModelRuntimeFailure {
        code: format!("model.native-resource.{suffix}"),
        retryable_after_correction: false,
        dependency_recovery_required: true,
        contract_error: None,
    }
}

fn bounded_text(path: &Path, maximum: u64) -> Result<String, ModelRuntimeFailure> {
    let mut text = String::new();
    File::open(path)
        .map_err(|_| failure("observation-unavailable"))?
        .take(maximum + 1)
        .read_to_string(&mut text)
        .map_err(|_| failure("observation-invalid"))?;
    if text.len() as u64 > maximum {
        return Err(failure("observation-invalid"));
    }
    Ok(text)
}

fn integer(value: &str) -> Result<u64, ModelRuntimeFailure> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(failure("observation-invalid"));
    }
    value.parse().map_err(|_| failure("observation-invalid"))
}

fn available_ram(text: &str) -> Result<u64, ModelRuntimeFailure> {
    let fields = text
        .lines()
        .filter_map(|line| line.strip_prefix("MemAvailable:"))
        .collect::<Vec<_>>();
    if fields.len() != 1 {
        return Err(failure("observation-invalid"));
    }
    let fields = fields[0].split_whitespace().collect::<Vec<_>>();
    if fields.len() != 2 || fields[1] != "kB" {
        return Err(failure("observation-invalid"));
    }
    integer(fields[0])?
        .checked_mul(1024)
        .ok_or_else(|| failure("observation-invalid"))
}

fn cgroup_path(text: &str) -> Result<String, ModelRuntimeFailure> {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() != 1 {
        return Err(failure("scope-unavailable"));
    }
    let path = lines[0]
        .strip_prefix("0::")
        .ok_or_else(|| failure("scope-unavailable"))?;
    if path == "/"
        || !path.starts_with('/')
        || path.contains('\0')
        || path
            .split('/')
            .skip(1)
            .any(|part| part.is_empty() || part == "." || part == "..")
        || Path::new(path)
            .components()
            .skip(1)
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(failure("scope-unavailable"));
    }
    Ok(path.into())
}

fn cpu_max(text: &str) -> Result<(u64, u64), ModelRuntimeFailure> {
    let fields = text.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 2 {
        return Err(failure("observation-invalid"));
    }
    Ok((integer(fields[0])?, integer(fields[1])?))
}

fn validate_scope(
    high: u64,
    max: u64,
    swap: u64,
    quota: u64,
    period: u64,
) -> Result<(), ModelRuntimeFailure> {
    if high == 0
        || high > 5 * GIB
        || max < high
        || max > 6 * GIB
        || swap > GIB / 2
        || quota == 0
        || period == 0
        || period.checked_mul(2).is_none_or(|limit| quota > limit)
    {
        return Err(failure("scope-denied"));
    }
    Ok(())
}

fn gpu_memory(text: &str) -> Result<(u64, u64, u64), ModelRuntimeFailure> {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() != 1 {
        return Err(failure("gpu-observation-invalid"));
    }
    let fields = lines[0].split(',').map(str::trim).collect::<Vec<_>>();
    if fields.len() != 3 {
        return Err(failure("gpu-observation-invalid"));
    }
    let used = integer(fields[0])?;
    let free = integer(fields[1])?;
    let total = integer(fields[2])?;
    // Device reserved memory can make used + free less than total.
    if total == 0 || used.checked_add(free).is_none_or(|sum| sum > total) {
        return Err(failure("gpu-observation-invalid"));
    }
    Ok((used, free, total))
}

fn validate_observation(value: &Observation) -> Result<(), ModelRuntimeFailure> {
    if value.observed_epoch_ms == 0
        || value.gpu_total_mib == 0
        || value
            .gpu_used_mib
            .checked_add(value.gpu_free_mib)
            .is_none_or(|sum| sum > value.gpu_total_mib)
    {
        return Err(failure("observation-invalid"));
    }
    validate_scope(
        value.memory_high_bytes,
        value.memory_max_bytes,
        value.memory_swap_max_bytes,
        value.cpu_quota_us,
        value.cpu_period_us,
    )?;
    if value.available_ram_bytes < 16 * GIB {
        return Err(failure("ram-contended"));
    }
    if value.gpu_free_mib < 21000 || value.gpu_used_mib > 22528 {
        return Err(failure("gpu-contended"));
    }
    Ok(())
}

fn query_gpu(expected_sha256: &str) -> Result<String, ModelRuntimeFailure> {
    let mut executable = File::from(
        open(
            NVIDIA_SMI,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| failure("observer-unavailable"))?,
    );
    let before = executable
        .metadata()
        .map_err(|_| failure("observer-invalid"))?;
    if !before.is_file()
        || before.uid() != 0
        || before.mode() & 0o7022 != 0
        || before.mode() & 0o100 == 0
        || before.len() == 0
        || before.len() > 16 * 1024 * 1024
    {
        return Err(failure("observer-invalid"));
    }
    let mut digest_state = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut read_bytes = 0u64;
    loop {
        let count = executable
            .read(&mut buffer)
            .map_err(|_| failure("observer-invalid"))?;
        if count == 0 {
            break;
        }
        read_bytes = read_bytes
            .checked_add(count as u64)
            .ok_or_else(|| failure("observer-invalid"))?;
        if read_bytes > before.len() {
            return Err(failure("observer-drift"));
        }
        digest_state.update(&buffer[..count]);
    }
    let after = executable
        .metadata()
        .map_err(|_| failure("observer-invalid"))?;
    let actual: String = digest_state
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if actual != expected_sha256
        || read_bytes != before.len()
        || before.len() != after.len()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.mtime() != after.mtime()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
    {
        return Err(failure("observer-drift"));
    }
    run_observer(&executable)
}

fn run_observer(executable: &File) -> Result<String, ModelRuntimeFailure> {
    // Execute the continuously held, hashed object, not a re-resolved path.
    let path = format!("/proc/{}/fd/{}", std::process::id(), executable.as_raw_fd());
    let mut child = Command::new(path)
        .args([
            "--query-gpu=memory.used,memory.free,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .env_clear()
        .env("LANG", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| failure("observer-unavailable"))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(failure("observer-failed"));
            }
        }
    };
    if !status.success() {
        return Err(failure("observer-failed"));
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| failure("observer-failed"))?;
    let flags = fcntl_getfl(&stdout).map_err(|_| failure("observer-failed"))?;
    fcntl_setfl(&stdout, flags | OFlags::NONBLOCK).map_err(|_| failure("observer-failed"))?;
    let mut text = String::new();
    stdout
        .take(4097)
        .read_to_string(&mut text)
        .map_err(|_| failure("observer-failed"))?;
    if text.len() > 4096 {
        return Err(failure("observer-failed"));
    }
    Ok(text)
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_observer_execution_is_cpu_only_and_has_bounded_empty_output() {
        // This is an execution-boundary regression, not a GPU observation. The
        // production caller supplies only the pinned observer with fixed arguments.
        let executable = File::open("/usr/bin/true").unwrap();
        assert_eq!(run_observer(&executable).unwrap(), "");
    }

    #[test]
    fn resource_policy_is_bound_to_the_unchanged_preparation_limits() {
        let bytes = include_bytes!("../../../model-profiles/development/coding-model-lab.json");
        assert!(NativeDevelopmentResourcePolicy::from_preparation_manifest(bytes).is_ok());
        for (field, value) in [
            ("context_tokens", 8192),
            ("threads", 8),
            ("minimum_available_ram_gib", 1),
            ("minimum_free_vram_mib", 1),
            ("maximum_total_vram_used_mib", 24000),
            ("parallel_slots", 2),
        ] {
            let mut changed: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            changed[field] = value.into();
            assert!(
                NativeDevelopmentResourcePolicy::from_preparation_manifest(
                    &serde_json::to_vec(&changed).unwrap()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn kernel_observation_parsers_refuse_ambiguity_and_unbounded_controls() {
        assert_eq!(
            available_ram("MemTotal: 123 kB\nMemAvailable: 16777216 kB\n").unwrap(),
            16 * GIB
        );
        for text in [
            "",
            "MemAvailable: 1 MB",
            "MemAvailable: 1 kB\nMemAvailable: 2 kB",
            "MemAvailable: 18446744073709551615 kB",
        ] {
            assert!(available_ram(text).is_err());
        }
        assert_eq!(
            cgroup_path("0::/user.slice/own.scope\n").unwrap(),
            "/user.slice/own.scope"
        );
        for text in [
            "0::/",
            "1:memory:/test",
            "0::/../escape",
            "0::/a//b",
            "0::/a\n0::/b",
        ] {
            assert!(cgroup_path(text).is_err());
        }
        for text in [
            "max 100000",
            "-1 100000",
            "200000 100000 extra",
            "18446744073709551616 1",
        ] {
            assert!(cpu_max(text).is_err());
        }
        assert!(validate_scope(5 * GIB, 6 * GIB, GIB / 2, 200000, 100000).is_ok());
        for (high, max, swap, quota, period) in [
            (6 * GIB, 6 * GIB, 0, 1, 1),
            (5 * GIB, 7 * GIB, 0, 1, 1),
            (5 * GIB, 6 * GIB, GIB, 1, 1),
            (5 * GIB, 6 * GIB, 0, 200001, 100000),
            (0, 0, 0, 0, 0),
        ] {
            assert!(validate_scope(high, max, swap, quota, period).is_err());
        }
    }

    #[test]
    fn single_gpu_and_inclusive_resource_boundaries_are_exact() {
        assert_eq!(
            gpu_memory("2000, 21000, 24564\n").unwrap(),
            (2000, 21000, 24564)
        );
        for text in [
            "",
            "2000, 21000, 24564\n2000, 21000, 24564\n",
            "N/A, 21000, 24564",
            "24000, 21000, 24564",
            "1, 2, 0",
        ] {
            assert!(gpu_memory(text).is_err());
        }
        let mut value = Observation {
            manifest_sha256: "a".repeat(64),
            observed_epoch_ms: 1,
            available_ram_bytes: 16 * GIB,
            gpu_used_mib: 2000,
            gpu_free_mib: 21000,
            gpu_total_mib: 24564,
            memory_high_bytes: 5 * GIB,
            memory_max_bytes: 6 * GIB,
            memory_swap_max_bytes: GIB / 2,
            cpu_quota_us: 200000,
            cpu_period_us: 100000,
        };
        assert!(validate_observation(&value).is_ok());
        value.available_ram_bytes -= 1;
        assert_eq!(
            validate_observation(&value).unwrap_err().code,
            "model.native-resource.ram-contended"
        );
        value.available_ram_bytes += 1;
        value.gpu_free_mib -= 1;
        assert_eq!(
            validate_observation(&value).unwrap_err().code,
            "model.native-resource.gpu-contended"
        );
    }
}
