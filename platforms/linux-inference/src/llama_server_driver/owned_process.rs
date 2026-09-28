//! Held process identities for this driver's runtime and isolated namespace init.

use std::fs;
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::process::Child;

use agentmage_kernel_contracts::ModelRuntimeFailure;
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::process::{Pid, PidfdFlags, pidfd_open};

use super::{
    descendant_of, exact_runtime_descendant, failure, process_parent, process_start_generation,
    status_value,
};

struct HeldProcess {
    descriptor: OwnedFd,
    pid: u32,
    generation: u64,
}

impl HeldProcess {
    fn capture(pid: u32) -> Result<Self, ModelRuntimeFailure> {
        let error = || failure("model.llama-driver.process-owner-unavailable", true);
        let generation = process_start_generation(pid)?;
        let raw = i32::try_from(pid)
            .ok()
            .and_then(Pid::from_raw)
            .ok_or_else(error)?;
        let held = Self {
            descriptor: pidfd_open(raw, PidfdFlags::empty()).map_err(|_| error())?,
            pid,
            generation,
        };
        held.verify_live()?;
        Ok(held)
    }

    fn exited(&self) -> Result<bool, ModelRuntimeFailure> {
        let mut descriptors = [PollFd::new(&self.descriptor, PollFlags::IN)];
        let zero = Timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        match poll(&mut descriptors, Some(&zero)) {
            Ok(0) => Ok(false),
            Ok(_)
                if descriptors[0]
                    .revents()
                    .intersects(PollFlags::IN | PollFlags::HUP)
                    && !descriptors[0]
                        .revents()
                        .intersects(PollFlags::ERR | PollFlags::NVAL) =>
            {
                Ok(true)
            }
            // Interrupted/error observations never manufacture an exit.
            _ => Err(failure(
                "model.llama-driver.process-owner-observation-failed",
                true,
            )),
        }
    }

    fn verify_live(&self) -> Result<(), ModelRuntimeFailure> {
        if self.exited()?
            || process_start_generation(self.pid)? != self.generation
            || self.exited()?
        {
            return Err(failure("model.llama-driver.process-owner-drift", true));
        }
        Ok(())
    }
}

pub(super) struct RuntimeOwner {
    runtime: HeldProcess,
    init: HeldProcess,
    namespace: PathBuf,
}

fn namespace(pid: u32) -> Result<PathBuf, ModelRuntimeFailure> {
    fs::read_link(format!("/proc/{pid}/ns/pid"))
        .map_err(|_| failure("model.llama-driver.namespace-owner-unavailable", true))
}

fn is_namespace_init(pid: u32) -> Result<bool, ModelRuntimeFailure> {
    let error = || failure("model.llama-driver.namespace-owner-unavailable", true);
    let status = fs::read_to_string(format!("/proc/{pid}/status")).map_err(|_| error())?;
    parse_namespace_init(&status, pid)
}

fn parse_namespace_init(status: &str, pid: u32) -> Result<bool, ModelRuntimeFailure> {
    let error = || failure("model.llama-driver.namespace-owner-unavailable", true);
    if status
        .lines()
        .filter(|line| line.starts_with("NSpid:"))
        .count()
        != 1
    {
        return Err(error());
    }
    let raw = status_value(status, "NSpid:").ok_or_else(error)?;
    let mut values = raw.split_whitespace().map(str::parse::<u32>);
    let first = values.next().ok_or_else(error)?.map_err(|_| error())?;
    if first != pid || first <= 1 {
        return Err(error());
    }
    let mut innermost = first;
    let mut count = 1;
    for value in values {
        innermost = value.map_err(|_| error())?;
        count += 1;
        if innermost == 0 || count > 32 {
            return Err(error());
        }
    }
    Ok(count > 1 && innermost == 1)
}

impl RuntimeOwner {
    /// No caller-provided PID: discover within the exact directly owned launcher.
    pub(super) fn capture(child: &mut Child) -> Result<Self, ModelRuntimeFailure> {
        let error = || failure("model.llama-driver.namespace-owner-unavailable", true);
        if child.try_wait().map_err(|_| error())?.is_some() {
            return Err(error());
        }
        let supervisor = child.id();
        let runtime = HeldProcess::capture(exact_runtime_descendant(supervisor)?)?;
        let isolated = namespace(runtime.pid)?;
        if isolated == namespace(std::process::id())? {
            return Err(error());
        }
        let mut candidate = runtime.pid;
        let mut init_pid = None;
        for _ in 0..32 {
            if namespace(candidate)? != isolated {
                break;
            }
            if is_namespace_init(candidate)? {
                init_pid = Some(candidate);
                break;
            }
            if candidate == supervisor {
                break;
            }
            candidate = process_parent(candidate)
                .filter(|parent| *parent > 1)
                .ok_or_else(error)?;
        }
        let init = HeldProcess::capture(init_pid.ok_or_else(error)?)?;
        let owner = Self {
            runtime,
            init,
            namespace: isolated,
        };
        if (owner.init.pid != supervisor && !descendant_of(owner.init.pid, supervisor))
            || exact_runtime_descendant(supervisor)? != owner.runtime.pid
            || child.try_wait().map_err(|_| error())?.is_some()
        {
            return Err(error());
        }
        owner.verify_live()?;
        Ok(owner)
    }

    pub(super) const fn pid(&self) -> u32 {
        self.runtime.pid
    }
    pub(super) const fn generation(&self) -> u64 {
        self.runtime.generation
    }

    pub(super) fn verify_live(&self) -> Result<(), ModelRuntimeFailure> {
        self.runtime.verify_live()?;
        self.init.verify_live()?;
        if namespace(self.runtime.pid)? != self.namespace
            || namespace(self.init.pid)? != self.namespace
            || !is_namespace_init(self.init.pid)?
            || (self.runtime.pid != self.init.pid
                && !descendant_of(self.runtime.pid, self.init.pid))
        {
            return Err(failure("model.llama-driver.namespace-owner-drift", true));
        }
        self.runtime.verify_live()?;
        self.init.verify_live()
    }

    pub(super) fn exited(&self) -> Result<bool, ModelRuntimeFailure> {
        Ok(self.runtime.exited()? && self.init.exited()?)
    }

    #[cfg(test)]
    pub(super) fn cpu_fixture(child: &Child) -> Result<Self, ModelRuntimeFailure> {
        // Test-only cleanup descriptor pair. This does not admit a namespace or
        // pass verify_live, and it cannot be constructed by production callers.
        Ok(Self {
            runtime: HeldProcess::capture(child.id())?,
            init: HeldProcess::capture(child.id())?,
            namespace: PathBuf::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{Duration, Instant};

    struct ChildFixture(Child);
    impl Drop for ChildFixture {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                if self.0.try_wait().unwrap().is_some() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            panic!("owned CPU fixture was not reaped");
        }
    }

    #[test]
    fn held_descriptor_reports_only_the_captured_process_exit() {
        let mut child = ChildFixture(
            Command::new("/usr/bin/sleep")
                .arg("30")
                .env_clear()
                .spawn()
                .unwrap(),
        );
        let mut held = HeldProcess::capture(child.0.id()).unwrap();
        assert!(!held.exited().unwrap());
        held.verify_live().unwrap();
        held.generation = held.generation.checked_add(1).unwrap();
        assert!(held.verify_live().is_err());
        held.generation -= 1;
        held.verify_live().unwrap();
        child.0.kill().unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !held.exited().unwrap() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        // Observe exit through the held descriptor before fallback fixture Drop.
        assert!(held.exited().unwrap());
        assert!(held.verify_live().is_err());
        while child.0.try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(child.0.try_wait().unwrap().is_some());
        assert!(held.exited().unwrap());
    }

    #[test]
    fn namespace_init_requires_exact_complete_nested_identity() {
        assert!(parse_namespace_init("NSpid:\t4321\t1\n", 4321).unwrap());
        assert!(parse_namespace_init("NSpid:\t4321 52 1\n", 4321).unwrap());
        assert!(!parse_namespace_init("NSpid:\t4321\n", 4321).unwrap());
        assert!(!parse_namespace_init("NSpid:\t4321 2\n", 4321).unwrap());
        for text in [
            "",
            "NSpid:",
            "NSpid: 4000 1",
            "NSpid: 4321 0",
            "NSpid: 4321 x",
            "NSpid: 4321 1\nNSpid: 4321 1",
        ] {
            assert!(parse_namespace_init(text, 4321).is_err(), "{text}");
        }
        assert!(parse_namespace_init(&format!("NSpid: 4321 {}", "1 ".repeat(32)), 4321).is_err());
    }
}
