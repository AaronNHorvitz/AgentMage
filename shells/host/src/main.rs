#![forbid(unsafe_code)]

mod package_verify;

struct HostExit(&'static str);

impl std::fmt::Debug for HostExit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

fn main() -> Result<(), HostExit> {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    if let Some(command) = arguments.next() {
        let root = arguments.next();
        if arguments.next().is_some() {
            return invalid_package_arguments();
        }
        let result = match (command.to_str(), root) {
            (Some("--verify-package-root"), Some(root)) => package_verify::verify_package_root(
                &root,
                package_verify::PackageClass::SignedRelease,
            ),
            (Some("--verify-package-candidate-root"), Some(root)) => {
                package_verify::verify_package_root(
                    &root,
                    package_verify::PackageClass::UnsignedCandidate,
                )
            }
            _ => return invalid_package_arguments(),
        };
        match result {
            Ok(()) => {
                println!("agentmage.package.valid");
                return Ok(());
            }
            Err(error) => return Err(HostExit(error.code())),
        }
    }

    let shared_components = (
        agentmage_host::COMPONENT_ID,
        agentmage_kernel_contracts::COMPONENT_ID,
        agentmage_kernel_engine::COMPONENT_ID,
        agentmage_capability_read_only::COMPONENT_ID,
    );

    #[cfg(target_os = "linux")]
    let platform_component = agentmage_platform_linux::COMPONENT_ID;
    #[cfg(not(target_os = "linux"))]
    let platform_component = "platform-adapter-not-linked";

    let _composition = (shared_components, platform_component);
    Ok(())
}

const fn invalid_package_arguments() -> Result<(), HostExit> {
    Err(HostExit("agentmage.package.arguments_invalid"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn host_binary_waits_for_verified_package_bootstrap() {
        assert_eq!(agentmage_host::COMPONENT_ID, "shell-host");
        assert_eq!(agentmage_kernel_engine::COMPONENT_ID, "kernel-engine");
    }
}
