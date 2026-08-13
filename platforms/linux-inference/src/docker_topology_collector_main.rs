#![forbid(unsafe_code)]

use std::process::ExitCode;

use agentmage_platform_linux_inference::{
    DOCKER_PREFLIGHT_CONTRACT_VERSION, DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION,
    observe_live_topology_from_reader, validate_topology_from_reader,
};

const SELF_CHECK: &str = "{\"accepted_operations\":[\"observe\",\"self-check\",\"validate-observation-stdin\"],\"authority\":\"docker-topology-observation-only\",\"component_id\":\"agentmage-docker-topology-collector\",\"docker_mutation\":false,\"enabled\":false,\"network_egress\":false,\"preflight_contract_version\":3,\"protocol_version\":2}\n";

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match arguments.as_slice() {
        [argument] if argument == "--self-check" => {
            print!("{SELF_CHECK}");
            ExitCode::SUCCESS
        }
        [argument] if argument == "--observe" => observe(),
        [argument] if argument == "--validate-observation-stdin" => validate(),
        _ => refuse("docker-collector.input"),
    }
}

fn observe() -> ExitCode {
    let uid = rustix::process::geteuid().as_raw();
    match observe_live_topology_from_reader(std::io::stdin().lock(), uid) {
        Ok(output) => write_output(&output),
        Err(error) => refuse(error.code()),
    }
}

fn validate() -> ExitCode {
    let uid = rustix::process::geteuid().as_raw();
    match validate_topology_from_reader(std::io::stdin().lock(), uid) {
        Ok(output) => write_output(&output),
        Err(error) => refuse(error.code()),
    }
}

fn write_output(output: &impl serde::Serialize) -> ExitCode {
    match serde_json::to_writer(std::io::stdout().lock(), output) {
        Ok(()) => {
            println!();
            ExitCode::SUCCESS
        }
        Err(_) => refuse("docker-collector.output"),
    }
}

fn refuse(code: &str) -> ExitCode {
    eprintln!("{code}");
    ExitCode::FAILURE
}

const _: u16 = DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION;
const _: u16 = DOCKER_PREFLIGHT_CONTRACT_VERSION;
