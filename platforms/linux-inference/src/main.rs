#![forbid(unsafe_code)]

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match agentmage_platform_linux_inference::evaluate_arguments(&arguments) {
        Ok(description) => {
            if std::io::stdout().lock().write_all(description).is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("{}", error.code());
            ExitCode::FAILURE
        }
    }
}
