#![forbid(unsafe_code)]

use std::process::ExitCode;

use agentmage_platform_linux_inference::{
    DOCKER_PREFLIGHT_CONTRACT_VERSION, DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION,
    validate_topology_from_reader,
};

const SELF_CHECK: &str = "{\"accepted_operations\":[\"self-check\",\"validate-observation-stdin\"],\"authority\":\"docker-topology-validation-only\",\"component_id\":\"agentmage-docker-topology-collector\",\"docker_mutation\":false,\"enabled\":false,\"network_egress\":false,\"preflight_contract_version\":2,\"protocol_version\":1}\n";

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match arguments.as_slice() {
        [argument] if argument == "--self-check" => {
            print!("{SELF_CHECK}");
            ExitCode::SUCCESS
        }
        [argument] if argument == "--validate-observation-stdin" => validate(),
        _ => refuse("docker-collector.input"),
    }
}

fn validate() -> ExitCode {
    let uid = rustix::process::geteuid().as_raw();
    match validate_topology_from_reader(std::io::stdin().lock(), uid) {
        Ok(output) => match serde_json::to_writer(std::io::stdout().lock(), &output) {
            Ok(()) => {
                println!();
                ExitCode::SUCCESS
            }
            Err(_) => refuse("docker-collector.output"),
        },
        Err(error) => refuse(error.code()),
    }
}

fn refuse(code: &str) -> ExitCode {
    eprintln!("{code}");
    ExitCode::FAILURE
}

const _: u16 = DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION;
const _: u16 = DOCKER_PREFLIGHT_CONTRACT_VERSION;
