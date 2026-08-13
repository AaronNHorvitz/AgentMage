#![forbid(unsafe_code)]

use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use agentmage_platform_linux_inference::{
    DOCKER_GUARD_BOOTSTRAP_BYTES, DOCKER_GUARD_PROTOCOL_VERSION, DockerGuardBootstrap,
    DockerGuardService,
};

const SOCKET_PATH: &str = "/run/agentmage-dmr/guard.sock";
const SELF_CHECK: &str = "{\"accepted_operations\":[\"serve-one-session\",\"self-check\"],\"authority\":\"guarded-inference-transport-only\",\"component_id\":\"agentmage-docker-guard\",\"docker_control\":false,\"enabled\":false,\"network_egress\":false,\"protocol_version\":1,\"raw_target\":\"private-namespace-loopback-only\",\"sessions\":1}\n";

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [argument] if argument == "--self-check" => {
            print!("{SELF_CHECK}");
            Ok(())
        }
        [argument] if argument == "--serve" => serve(),
        _ => Err("docker-guard.service.bootstrap"),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("{code}");
            ExitCode::FAILURE
        }
    }
}

fn serve() -> Result<(), &'static str> {
    let mut frame = [0_u8; DOCKER_GUARD_BOOTSTRAP_BYTES];
    std::io::stdin()
        .lock()
        .read_exact(&mut frame)
        .map_err(|_| "docker-guard.service.bootstrap")?;
    let bootstrap = DockerGuardBootstrap::decode(&frame).map_err(|error| error.code())?;
    let service = DockerGuardService::bind(Path::new(SOCKET_PATH), bootstrap)
        .map_err(|error| error.code())?;
    service.serve_once().map_err(|error| error.code())
}

const _: u16 = DOCKER_GUARD_PROTOCOL_VERSION;
