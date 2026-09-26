//! Operation-scoped worker; the parent owns grants, admission, confinement and receipts.
//! No host registration or production activation is supplied by this optional binary.
#![forbid(unsafe_code)]

#[path = "../public_research_target.rs"]
mod target;
#[path = "../public_research_transport.rs"]
mod transport;

use std::io::{Read, Write};
use std::process::ExitCode;

use agentmage_kernel_engine::research_fetch::PublicGetWorkerPacket;

fn run() -> Result<(), transport::WorkerError> {
    if std::env::args_os().count() != 1 || !environment_is_admitted(std::env::vars_os()) {
        return Err(transport::WorkerError::Environment);
    }
    // Defense in depth, not proof of the exact sandbox. The independently admitted
    // parent must verify the full namespace/filter/runtime manifest before dispatch.
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|_| transport::WorkerError::Environment)?;
    if !status.lines().any(|line| line == "NoNewPrivs:\t1")
        || !status.lines().any(|line| line == "Seccomp:\t2")
    {
        return Err(transport::WorkerError::Environment);
    }
    // The sandbox uses stdin to install seccomp. The request is instead an exact
    // sealed descriptor projected read-only at this fixed path, never a workspace.
    let bytes = read_request(std::path::Path::new("/input/request"))?;
    let now = transport::epoch_ms()?;
    let packet = PublicGetWorkerPacket::decode_worker_packet(&bytes, now)
        .map_err(|_| transport::WorkerError::Input)?;
    let response = transport::execute(&packet)?;
    std::io::stdout()
        .lock()
        .write_all(response.frame())
        .map_err(|_| transport::WorkerError::Output)
}

fn read_request(path: &std::path::Path) -> Result<Vec<u8>, transport::WorkerError> {
    let input = std::fs::File::from(
        rustix::fs::open(
            path,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC
                | rustix::fs::OFlags::NONBLOCK,
            rustix::fs::Mode::empty(),
        )
        .map_err(|_| transport::WorkerError::Input)?,
    );
    let metadata = input
        .metadata()
        .map_err(|_| transport::WorkerError::Input)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 16 * 1024 {
        return Err(transport::WorkerError::Input);
    }
    let mut bytes = Vec::new();
    input
        .take(16 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| transport::WorkerError::Input)?;
    if bytes.len() as u64 != metadata.len() || bytes.len() > 16 * 1024 {
        return Err(transport::WorkerError::Input);
    }
    Ok(bytes)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Never format a URL, library error, query, environment or response body.
            eprintln!("{}", error.code());
            ExitCode::from(5)
        }
    }
}

fn environment_is_admitted(
    values: impl IntoIterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    for (key, value) in values {
        // Bubblewrap 0.12.0 sets PWD after --clearenv. It must be the fixed guest
        // working directory, never an ambient host path. Optional locale/timezone
        // values are also fixed, not arbitrary environment strings or secret input.
        let accepted = match key.to_str() {
            Some("LANG" | "LC_ALL") => value == "C",
            Some("TZ") => value == "UTC",
            Some("PWD") => value == "/input",
            _ => false,
        };
        if !accepted || !seen.insert(key) {
            return false;
        }
    }
    seen.contains(std::ffi::OsStr::new("LANG")) && seen.contains(std::ffi::OsStr::new("PWD"))
}

#[cfg(test)]
mod tests {
    use super::environment_is_admitted;
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    fn valid() -> Vec<(OsString, OsString)> {
        vec![("LANG".into(), "C".into()), ("PWD".into(), "/input".into())]
    }

    #[test]
    fn fixed_request_reader_denies_fifo_symlink_directory_empty_and_oversize() {
        use std::os::unix::fs::symlink;
        use std::time::{Instant, SystemTime, UNIX_EPOCH};
        let root = std::env::temp_dir().join(format!(
            "agentmage-worker-input-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("request");
        std::fs::write(&file, b"bounded request").unwrap();
        assert_eq!(super::read_request(&file).unwrap(), b"bounded request");
        assert!(super::read_request(&root).is_err());
        let link = root.join("link");
        symlink(&file, &link).unwrap();
        assert!(super::read_request(&link).is_err());
        let fifo = root.join("fifo");
        rustix::fs::mknodat(
            rustix::fs::CWD,
            &fifo,
            rustix::fs::FileType::Fifo,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
            0,
        )
        .unwrap();
        let started = Instant::now();
        assert!(super::read_request(&fifo).is_err());
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        std::fs::write(&file, []).unwrap();
        assert!(super::read_request(&file).is_err());
        std::fs::write(&file, vec![b'x'; 16 * 1024 + 1]).unwrap();
        assert!(super::read_request(&file).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_bubblewrap_environment_is_admitted_without_host_variables() {
        assert!(environment_is_admitted(valid()));
        let mut canonical = valid();
        canonical.extend([("LC_ALL".into(), "C".into()), ("TZ".into(), "UTC".into())]);
        assert!(environment_is_admitted(canonical));
    }

    #[test]
    fn missing_duplicated_noncanonical_or_non_unicode_environment_is_denied() {
        for index in 0..2 {
            let mut missing = valid();
            missing.remove(index);
            assert!(!environment_is_admitted(missing));
        }
        for (key, value) in [
            ("LANG", "C"), // duplicate, even when both values are identical
            ("PWD", "/input"),
            ("PWD", "/workspace"),
            ("PWD", "/input/.."),
            ("PATH", "/app"),
            ("HOME", "/tmp"),
            ("HTTPS_PROXY", "https://proxy.example.com"),
            ("SSL_CERT_FILE", "/input/roots"),
            ("LC_ALL", "en_US.UTF-8"),
            ("TZ", ":/input/zone"),
        ] {
            let mut bad = valid();
            bad.push((key.into(), value.into()));
            assert!(!environment_is_admitted(bad));
        }
        for bytes in [vec![b'L', 0xff], vec![0xff]] {
            let mut bad = valid();
            bad.push((OsString::from_vec(bytes.clone()), "C".into()));
            assert!(!environment_is_admitted(bad));
            let mut bad = valid();
            bad[0].1 = OsString::from_vec(bytes);
            assert!(!environment_is_admitted(bad));
        }
        for replacement in ["", "C.UTF-8", "private-canary"] {
            let mut bad = valid();
            bad[0].1 = replacement.into();
            assert!(!environment_is_admitted(bad));
        }
        for replacement in ["", "/", "/input/", "/input/../input", "/workspace"] {
            let mut bad = valid();
            bad[1].1 = replacement.into();
            assert!(!environment_is_admitted(bad));
        }
    }
}
