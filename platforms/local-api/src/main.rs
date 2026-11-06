// SPDX-License-Identifier: Apache-2.0
//! Local API process with explicit source scopes and separately approved writes.

use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};

use agentmage_local_api::{ApiTokens, serve};

fn main() {
    if run().is_err() {
        eprintln!(
            "Local API failed. Check the loopback address and private authentication directory."
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
        _ => {
            eprintln!(
                "Usage: agentmage-local-api init-auth <new-private-directory> | serve <loopback-ip:port> <private-directory>"
            );
            Err(())
        }
    }
}
