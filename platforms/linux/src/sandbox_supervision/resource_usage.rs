//! Optional bounded accounting through the supervisor's already held cgroup.
//! No process lookup, authority, admission decision or independent sampler.

use agentmage_kernel_engine::command_runner::CommandResourceUsage;
use rustix::fd::OwnedFd;
use rustix::fs::{Mode, OFlags, openat};
use rustix::io::pread;
use std::collections::BTreeSet;

const ACCOUNTING_BYTES: usize = 4096;

fn bounded_read(directory: &OwnedFd, name: &str) -> Option<Vec<u8>> {
    let descriptor = openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .ok()?;
    let mut bytes = [0; ACCOUNTING_BYTES + 1];
    let count = pread(&descriptor, &mut bytes, 0).ok()?;
    (count > 0 && count <= ACCOUNTING_BYTES).then(|| bytes[..count].to_vec())
}

fn number(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() || bytes.len() > 20 || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

fn scalar(bytes: &[u8]) -> Option<u64> {
    number(bytes.strip_suffix(b"\n")?)
}

fn cpu_nanoseconds(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty()
        || bytes.len() > ACCOUNTING_BYTES
        || !bytes.ends_with(b"\n")
        || bytes.iter().any(|b| b.is_ascii_control() && *b != b'\n')
    {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let mut names = BTreeSet::new();
    let mut usage = None;
    for line in text.lines() {
        let (name, value) = line.split_once(' ')?;
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'_' || b == b'.')
            || !names.insert(name)
        {
            return None;
        }
        // Kernel fields may be added. Require every field to be distinct and
        // well-formed, but only usage_usec has the documented conversion here.
        let value = number(value.as_bytes())?;
        if name == "usage_usec" {
            usage = Some(value.checked_mul(1000)?);
        }
    }
    usage
}

fn parse(cpu: &[u8], memory: &[u8], tasks: &[u8]) -> Option<CommandResourceUsage> {
    Some(CommandResourceUsage {
        cpu_time_ns: cpu_nanoseconds(cpu)?,
        peak_memory_bytes: scalar(memory)?,
        peak_task_count: u32::try_from(scalar(tasks)?).ok()?,
    })
}

pub(super) fn sample(directory: &OwnedFd) -> Option<CommandResourceUsage> {
    // The caller must supply only its descriptor-held verified cgroup-v2
    // directory. Removal, missing counters or malformed data are unavailable,
    // never fabricated zero use. Reads are bounded and contain no subprocess.
    parse(
        &bounded_read(directory, "cpu.stat")?,
        &bounded_read(directory, "memory.peak")?,
        &bounded_read(directory, "pids.current")?,
    )
}

#[derive(Default)]
pub(super) struct Observation {
    value: Option<CommandResourceUsage>,
}

impl Observation {
    pub(super) fn merge(&mut self, sample: Option<CommandResourceUsage>) {
        let Some(sample) = sample else { return };
        self.value = Some(match self.value.take() {
            None => sample,
            Some(previous) => CommandResourceUsage {
                cpu_time_ns: previous.cpu_time_ns.max(sample.cpu_time_ns),
                peak_memory_bytes: previous.peak_memory_bytes.max(sample.peak_memory_bytes),
                peak_task_count: previous.peak_task_count.max(sample.peak_task_count),
            },
        });
    }

    pub(super) fn finish(self) -> Option<CommandResourceUsage> {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_directory_samples_are_bounded_and_missing_removed_or_symlinked_is_unavailable() {
        use rustix::fs::open;
        use rustix::rand::{GetRandomFlags, getrandom};
        use std::fs;
        let mut nonce = [0_u8; 12];
        getrandom(&mut nonce, GetRandomFlags::empty()).unwrap();
        let name: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
        let root = std::env::temp_dir().join(format!("agentmage-accounting-{name}"));
        fs::create_dir(&root).unwrap();
        // Synthetic descriptor/IO proof, not cgroup admission or native measurement.
        let directory = open(
            &root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .unwrap();
        assert!(sample(&directory).is_none());
        fs::write(root.join("cpu.stat"), b"usage_usec 2\n").unwrap();
        fs::write(root.join("memory.peak"), b"10\n").unwrap();
        fs::write(root.join("pids.current"), b"1\n").unwrap();
        assert_eq!(sample(&directory).unwrap().cpu_time_ns, 2000);
        fs::write(root.join("cpu.stat"), vec![b'1'; ACCOUNTING_BYTES + 1]).unwrap();
        assert!(sample(&directory).is_none());
        fs::remove_file(root.join("cpu.stat")).unwrap();
        std::os::unix::fs::symlink("memory.peak", root.join("cpu.stat")).unwrap();
        assert!(sample(&directory).is_none());
        fs::remove_file(root.join("cpu.stat")).unwrap();
        fs::remove_file(root.join("memory.peak")).unwrap();
        fs::remove_file(root.join("pids.current")).unwrap();
        fs::remove_dir(&root).unwrap();
        assert!(sample(&directory).is_none());
    }

    #[test]
    fn accounting_uses_total_cpu_and_checked_units() {
        let result = parse(
            b"usage_usec 123\nuser_usec 100\nsystem_usec 23\nnr_periods 2\n",
            b"456\n",
            b"3\n",
        )
        .unwrap();
        assert_eq!(result.cpu_time_ns, 123_000);
        assert_eq!(result.peak_memory_bytes, 456);
        assert_eq!(result.peak_task_count, 3);
        assert!(parse(b"usage_usec 18446744073709552\n", b"0\n", b"0\n").is_none());
    }

    #[test]
    fn missing_malformed_duplicate_and_partial_accounting_is_unavailable() {
        for cpu in [
            b"".as_slice(),
            b"usage_usec 1",
            b"usage_usec +1\n",
            b"usage_usec 1\nusage_usec 2\n",
            b"user_usec 1\n",
            b"usage_usec 1\n\n",
            b"usage_usec 1\r\n",
            b"usage_usec 1\nunknown NaN\n",
            b"usage_usec 1\nfoo 1\nfoo 2\n",
            b"usage_usec 1\nfoo\t 2\n",
        ] {
            assert!(parse(cpu, b"1\n", b"1\n").is_none(), "{cpu:?}");
        }
        for scalar_value in [
            b"".as_slice(),
            b"1",
            b"1\n2\n",
            b"max\n",
            b"-1\n",
            b"+1\n",
            b"1 \n",
            b"18446744073709551616\n",
        ] {
            assert!(parse(b"usage_usec 1\n", scalar_value, b"1\n").is_none());
            assert!(parse(b"usage_usec 1\n", b"1\n", scalar_value).is_none());
        }
        assert!(parse(b"usage_usec 1\n", b"1\n", b"4294967296\n").is_none());
        assert!(cpu_nanoseconds(&vec![b'a'; ACCOUNTING_BYTES + 1]).is_none());
    }

    #[test]
    fn unavailable_sample_never_invents_zero_or_erases_prior_observation() {
        let mut values = Observation::default();
        values.merge(None);
        assert!(values.value.is_none());
        values.merge(parse(b"usage_usec 3\n", b"20\n", b"2\n"));
        values.merge(None);
        values.merge(parse(b"usage_usec 4\n", b"10\n", b"1\n"));
        let result = values.finish().unwrap();
        assert_eq!(result.cpu_time_ns, 4000);
        assert_eq!(result.peak_memory_bytes, 20);
        assert_eq!(result.peak_task_count, 2);
        let zero = parse(b"usage_usec 0\n", b"0\n", b"0\n").unwrap();
        assert_eq!(zero.cpu_time_ns, 0);
    }
}
