#![forbid(unsafe_code)]

use std::process::ExitCode;

use agentmage_host::cli::{
    CLI_VERSION, CliInvocation, CliOutputFormat, command_help, parse_cli_arguments,
    render_cli_error, shell_completion, unavailable_exit_code,
};
use agentmage_host::headless::ThinClientError;

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let invocation = match parse_cli_arguments(&arguments) {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("{}", render_cli_error(error, selected_format(&arguments)));
            return ExitCode::from(error.exit_code().process_code());
        }
    };
    match invocation {
        CliInvocation::Help => {
            print!("{}", command_help());
            ExitCode::SUCCESS
        }
        CliInvocation::Version => {
            println!("agent {CLI_VERSION}");
            ExitCode::SUCCESS
        }
        CliInvocation::Completion(shell) => {
            print!("{}", shell_completion(shell));
            ExitCode::SUCCESS
        }
        CliInvocation::Execute { output, .. } => {
            let error = ThinClientError::TransportFailed;
            eprintln!("{}", render_cli_error(error, output));
            ExitCode::from(unavailable_exit_code().process_code())
        }
    }
}

fn selected_format(arguments: &[String]) -> CliOutputFormat {
    if arguments.iter().any(|argument| argument == "--json") {
        CliOutputFormat::Json
    } else {
        CliOutputFormat::Human
    }
}
