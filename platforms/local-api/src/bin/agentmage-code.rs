// SPDX-License-Identifier: Apache-2.0
//! Terminal entry point for the local coding API client.

fn main() {
    if agentmage_local_api::coding_cli::run(&std::env::args().skip(1).collect::<Vec<_>>()).is_err()
    {
        eprintln!(
            "Coding request failed or was declined. Inspect the API status and files before retrying an action. No action is retried automatically."
        );
        std::process::exit(1);
    }
}
