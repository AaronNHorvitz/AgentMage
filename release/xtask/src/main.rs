#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::process::{Command, ExitCode};

const USAGE: &str =
    "usage: cargo run -p agentmage-xtask -- package-candidate [--version X.Y.Z] [--output PATH]";

fn main() -> ExitCode {
    match run(std::env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("{code}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: Vec<OsString>) -> Result<(), &'static str> {
    let forwarded = parse_arguments(&arguments)?;
    let root = workspace_root()?;
    let status = Command::new("python3")
        .arg(root.join("scripts/package_candidate.py"))
        .args(forwarded)
        .current_dir(root)
        .status()
        .map_err(|_| "release.package_builder_unavailable")?;
    status
        .success()
        .then_some(())
        .ok_or("release.package_candidate_failed")
}

fn parse_arguments(arguments: &[OsString]) -> Result<Vec<OsString>, &'static str> {
    if arguments.first().and_then(|value| value.to_str()) != Some("package-candidate") {
        eprintln!("{USAGE}");
        return Err("release.arguments_invalid");
    }
    let mut forwarded = Vec::new();
    let mut cursor = 1;
    while cursor < arguments.len() {
        let option = arguments[cursor]
            .to_str()
            .ok_or("release.arguments_invalid")?;
        if !matches!(option, "--version" | "--output") || cursor + 1 >= arguments.len() {
            return Err("release.arguments_invalid");
        }
        let value = arguments[cursor + 1]
            .to_str()
            .ok_or("release.arguments_invalid")?;
        if value.is_empty() || value.starts_with('-') {
            return Err("release.arguments_invalid");
        }
        forwarded.push(arguments[cursor].clone());
        forwarded.push(arguments[cursor + 1].clone());
        cursor += 2;
    }
    Ok(forwarded)
}

fn workspace_root() -> Result<std::path::PathBuf, &'static str> {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
        .ok_or("release.workspace_root_unavailable")
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::parse_arguments;

    #[test]
    fn package_candidate_forwards_only_closed_options() {
        let arguments = [
            "package-candidate",
            "--version",
            "0.0.1",
            "--output",
            "release-output",
        ]
        .map(OsString::from);
        assert_eq!(
            parse_arguments(&arguments).expect("valid command"),
            arguments[1..]
        );
    }

    #[test]
    fn unknown_missing_and_option_like_values_fail_closed() {
        for arguments in [
            vec!["unknown"],
            vec!["package-candidate", "--version"],
            vec!["package-candidate", "--unknown", "value"],
            vec!["package-candidate", "--output", "--version"],
        ] {
            let arguments: Vec<OsString> = arguments.into_iter().map(OsString::from).collect();
            assert!(parse_arguments(&arguments).is_err());
        }
    }
}
