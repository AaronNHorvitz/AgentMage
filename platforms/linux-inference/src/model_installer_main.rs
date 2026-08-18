#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match agentmage_platform_linux_inference::execute_model_installer(
        &arguments,
        std::io::stdin().lock(),
        std::io::stdout().lock(),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error.code());
            ExitCode::FAILURE
        }
    }
}
