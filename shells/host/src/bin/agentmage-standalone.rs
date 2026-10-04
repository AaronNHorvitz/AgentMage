//! The standalone application process (Decision 0150).
//!
//! Its window is not built: the secured webview it hosts needs a dependency
//! this build does not have, so starting it plainly reports that and exits.
//! `--development-stdio <state-root> <disposable-root>` runs the bridge to a
//! standalone evidence development host with the development presentation:
//! closed JSON request lines on standard input, closed JSON event lines on
//! standard output. That presentation is a message channel for development
//! and tests, not the product's presentation.
#![forbid(unsafe_code)]

use std::process::ExitCode;

const PRESENTATION_UNAVAILABLE: &str = "standalone.presentation.unavailable";
const USAGE: &str = "usage: agentmage-standalone --development-stdio <state-root> <disposable-root> [--fixture-step-delay-ms <0-2000>]";

fn main() -> ExitCode {
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    match arguments.as_slice() {
        [] => {
            eprintln!("{PRESENTATION_UNAVAILABLE}");
            ExitCode::from(3)
        }
        [mode, state_root, disposable_root] if mode == "--development-stdio" => {
            development_stdio(state_root.as_ref(), disposable_root.as_ref(), 0)
        }
        [mode, state_root, disposable_root, flag, delay]
            if mode == "--development-stdio" && flag == "--fixture-step-delay-ms" =>
        {
            match delay
                .to_str()
                .filter(|text| text == &"0" || !text.starts_with('0'))
                .and_then(|text| text.parse::<u16>().ok())
                .filter(|delay| *delay <= 2_000)
            {
                Some(delay) => {
                    development_stdio(state_root.as_ref(), disposable_root.as_ref(), delay)
                }
                None => {
                    eprintln!("{USAGE}");
                    ExitCode::from(2)
                }
            }
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(all(
    target_os = "linux",
    feature = "interactive-cli",
    feature = "native-chat",
    feature = "source-artifacts"
))]
fn development_stdio(
    state_root: &std::path::Path,
    disposable_root: &std::path::Path,
    step_delay_ms: u16,
) -> ExitCode {
    if !state_root.is_absolute() || !disposable_root.is_absolute() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    agentmage_host::standalone_shell::run_line_presentation(
        state_root,
        disposable_root,
        step_delay_ms,
        std::io::BufReader::new(std::io::stdin()),
        std::io::stdout(),
    );
    ExitCode::SUCCESS
}

#[cfg(not(all(
    target_os = "linux",
    feature = "interactive-cli",
    feature = "native-chat",
    feature = "source-artifacts"
)))]
fn development_stdio(
    _state_root: &std::path::Path,
    _disposable_root: &std::path::Path,
    _step_delay_ms: u16,
) -> ExitCode {
    eprintln!("{PRESENTATION_UNAVAILABLE}");
    ExitCode::from(3)
}
