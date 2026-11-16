// SPDX-License-Identifier: MIT OR Apache-2.0
//! Client-credential-only stdio MCP entry point over the local coding API.

fn main() {
    if agentmage_local_api::coding_cli::run_mcp(&std::env::args().skip(1).collect::<Vec<_>>())
        .is_err()
    {
        eprintln!(
            "MCP session refused or disconnected. Inspect the local API before retrying; no action was retried."
        );
        std::process::exit(1);
    }
}
