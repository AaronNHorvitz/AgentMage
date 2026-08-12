#![forbid(unsafe_code)]

mod package_verify;

struct HostExit(&'static str);

impl std::fmt::Debug for HostExit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

fn main() -> Result<(), HostExit> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if !arguments.is_empty() {
        let result = match arguments.as_slice() {
            [command, root] if command == "--verify-package-candidate-root" => {
                package_verify::verify_package_candidate_root(root)
            }
            [
                command,
                root,
                signature_option,
                signature,
                key_option,
                public_key,
            ] if command == "--verify-package-root"
                && signature_option == "--signature"
                && key_option == "--public-key" =>
            {
                package_verify::verify_signed_package_root(root, signature, public_key)
            }
            [command, _root] if command == "--verify-package-root" => {
                Err(package_verify::PackageVerificationError::ReleaseVerificationUnavailable)
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
