// SPDX-License-Identifier: Apache-2.0
//! Local API process with explicit source scopes and separately approved writes.

use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};

use agentmage_local_api::{
    ApiTokens, CodingRuntimeDevelopment, serve, serve_coding, serve_coding_runtime_development,
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
            serve_coding(
                listener,
                tokens,
                Arc::new(AtomicBool::new(false)),
                Path::new(state_root),
            )
            .map_err(|_| ())
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
            println!(
                "AgentMage coding runtime starting on {}",
                listener.local_addr().map_err(|_| ())?
            );
            eprintln!(
                "Starting the existing development host: repository inspection and private state; model and tools require separate approval."
            );
            let result = serve_coding_runtime_development(
                listener,
                tokens,
                stop,
                CodingRuntimeDevelopment {
                    state_root: Path::new(state),
                    disposable_root: Path::new(disposable),
                    workspace_root: Path::new(workspace),
                    model,
                    resume,
                },
            )
            .map_err(|_| ());
            for id in signals {
                signal_hook::low_level::unregister(id);
            }
            result
        }
        _ => {
            eprintln!(
                "Usage: agentmage-local-api init-auth <new-private-directory> | serve <loopback-ip:port> <private-directory> | serve-coding <loopback-ip:port> <private-directory> <existing-private-state-root> | serve-runtime-development <loopback-ip:port> <private-directory> <state-root> <disposable-root> <marked-workspace> <muse|gpt-oss> <new|resume>"
            );
            Err(())
        }
    }
}
