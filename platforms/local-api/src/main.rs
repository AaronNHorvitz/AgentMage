// SPDX-License-Identifier: Apache-2.0
//! Local API process with explicit source scopes and separately approved writes.

use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};

use agentmage_local_api::{
    ApiTokens, CodingRuntimeDevelopment, serve, serve_coding,
    serve_coding_endpoint_coding_development, serve_coding_endpoint_edit_development,
    serve_coding_endpoint_read_development, serve_coding_runtime_development,
};

fn main() {
    if run().is_err() {
        eprintln!(
            "Local API failed. Check the loopback address, private authentication directory and coding state directory."
        );
        std::process::exit(1);
    }
}

fn run() -> Result<(), ()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, directory] if command == "init-auth" => {
            ApiTokens::initialize(Path::new(directory)).map_err(|_| ())
        }
        [command, address, directory] if command == "serve" => {
            let address = address.parse::<SocketAddr>().map_err(|_| ())?;
            if !address.ip().is_loopback() {
                return Err(());
            }
            let tokens = ApiTokens::load(Path::new(directory)).map_err(|_| ())?;
            let listener = TcpListener::bind(address).map_err(|_| ())?;
            println!(
                "AgentMage local API listening on {}",
                listener.local_addr().map_err(|_| ())?
            );
            serve(listener, tokens, Arc::new(AtomicBool::new(false))).map_err(|_| ())
        }
        [command, address, directory, state_root] if command == "serve-coding" => {
            let address = address.parse::<SocketAddr>().map_err(|_| ())?;
            if !address.ip().is_loopback() {
                return Err(());
            }
            let tokens = ApiTokens::load(Path::new(directory)).map_err(|_| ())?;
            let listener = TcpListener::bind(address).map_err(|_| ())?;
            println!(
                "AgentMage local API starting on {}",
                listener.local_addr().map_err(|_| ())?
            );
            with_stop_signals(|stop| {
                serve_coding(listener, tokens, stop, Path::new(state_root)).map_err(|_| ())
            })
        }
        [command, address, directory, launch, approval]
            if command == "serve-endpoint-read-development"
                && approval == "--approve-workspace-state-and-endpoint" =>
        {
            let address = address.parse::<SocketAddr>().map_err(|_| ())?;
            if !address.ip().is_loopback() {
                return Err(());
            }
            let tokens = ApiTokens::load(Path::new(directory)).map_err(|_| ())?;
            let listener = TcpListener::bind(address).map_err(|_| ())?;
            eprintln!(
                "Starting a disposable read-only endpoint run: read private configuration and key, select the marked workspace and create private state. Each model disclosure and file read needs separate approval. Native admission must succeed."
            );
            eprintln!(
                "AgentMage endpoint API starting on {}",
                listener.local_addr().map_err(|_| ())?
            );
            with_stop_signals(|stop| {
                serve_coding_endpoint_read_development(listener, tokens, stop, Path::new(launch))
                    .map_err(|_| ())
            })
        }
        [command, address, directory, launch, approval]
            if (command == "serve-endpoint-edit-development"
                || command == "serve-endpoint-coding-development")
                && approval == "--approve-inspection-state-and-endpoint" =>
        {
            let address = address.parse::<SocketAddr>().map_err(|_| ())?;
            if !address.ip().is_loopback() {
                return Err(());
            }
            let tokens = ApiTokens::load(Path::new(directory)).map_err(|_| ())?;
            let listener = TcpListener::bind(address).map_err(|_| ())?;
            let coding = command == "serve-endpoint-coding-development";
            if coding {
                eprintln!(
                    "Starting disposable strict edits and fixed tests: inspect the entire repository with native offline Git and create private state. Worktree creation requires its exact stdin decision; each model disclosure, edit and test requires separate HTTP approval. No verified completion, apply, commit, push or recovery."
                );
            } else {
                eprintln!(
                    "Starting disposable strict edits: inspect the entire repository with native offline Git and create private state. Worktree creation requires its exact stdin decision; each model disclosure and edit requires separate HTTP approval. No tests, apply, commit, push or recovery."
                );
            }
            eprintln!(
                "AgentMage endpoint API starting on {}",
                listener.local_addr().map_err(|_| ())?
            );
            with_stop_signals(|stop| {
                let serve = if coding {
                    serve_coding_endpoint_coding_development
                } else {
                    serve_coding_endpoint_edit_development
                };
                serve(listener, tokens, stop, Path::new(launch)).map_err(|_| ())
            })
        }
        [
            command,
            address,
            directory,
            state,
            disposable,
            workspace,
            model,
            continuation,
        ]
        | [
            command,
            address,
            directory,
            state,
            disposable,
            workspace,
            model,
            continuation,
            _,
        ] if command == "serve-runtime-development" => {
            let address = address.parse::<SocketAddr>().map_err(|_| ())?;
            if !address.ip().is_loopback() {
                return Err(());
            }
            let resume = match continuation.as_str() {
                "new" => false,
                "resume" => true,
                _ => return Err(()),
            };
            let tokens = ApiTokens::load(Path::new(directory)).map_err(|_| ())?;
            let listener = TcpListener::bind(address).map_err(|_| ())?;
            println!(
                "AgentMage coding runtime starting on {}",
                listener.local_addr().map_err(|_| ())?
            );
            eprintln!(
                "Starting the existing development host: repository inspection and private state; model and tools require separate approval."
            );
            with_stop_signals(|stop| {
                serve_coding_runtime_development(
                    listener,
                    tokens,
                    stop,
                    CodingRuntimeDevelopment {
                        state_root: Path::new(state),
                        disposable_root: Path::new(disposable),
                        workspace_root: Path::new(workspace),
                        model,
                        scenario: args.get(8).map_or("failed-test-repair", String::as_str),
                        resume,
                    },
                )
                .map_err(|_| ())
            })
        }
        _ => {
            eprintln!(
                "Usage: agentmage-local-api init-auth <new-private-directory> | serve <loopback-ip:port> <private-directory> | serve-coding <loopback-ip:port> <private-directory> <existing-private-state-root> | serve-endpoint-read-development <loopback-ip:port> <private-directory> <private-launch.json> --approve-workspace-state-and-endpoint | serve-endpoint-edit-development <loopback-ip:port> <private-directory> <private-launch.json> --approve-inspection-state-and-endpoint | serve-endpoint-coding-development <loopback-ip:port> <private-directory> <private-launch-with-validation.json> --approve-inspection-state-and-endpoint | serve-runtime-development <loopback-ip:port> <private-directory> <state-root> <disposable-root> <marked-workspace> <muse|gpt-oss> <new|resume> [failed-test-repair|python-project|python-root-project]"
            );
            Err(())
        }
    }
}

fn with_stop_signals(operation: impl FnOnce(Arc<AtomicBool>) -> Result<(), ()>) -> Result<(), ()> {
    let stop = Arc::new(AtomicBool::new(false));
    let mut signals = Vec::new();
    for signal in [
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
    ] {
        match signal_hook::flag::register(signal, stop.clone()) {
            Ok(id) => signals.push(id),
            Err(_) => {
                for id in signals {
                    signal_hook::low_level::unregister(id);
                }
                return Err(());
            }
        }
    }
    let result = operation(stop);
    for id in signals {
        signal_hook::low_level::unregister(id);
    }
    result
}
