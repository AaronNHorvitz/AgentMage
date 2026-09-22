#![forbid(unsafe_code)]

mod package_verify;

#[cfg(target_os = "linux")]
mod linux_bootstrap;
#[cfg(target_os = "linux")]
mod model_catalog_bootstrap;

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
            #[cfg(target_os = "linux")]
            [command] if command == "--bootstrap-linux" => return bootstrap_linux(),
            #[cfg(target_os = "linux")]
            [
                command,
                state_root,
                disposable_root,
                workspace_root,
                scenario,
                model,
            ] if command == "--coding-development-host" => {
                return coding_development_host(
                    state_root,
                    disposable_root,
                    workspace_root,
                    scenario,
                    model,
                );
            }
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

#[cfg(target_os = "linux")]
fn coding_development_host(
    state_root: &std::ffi::OsStr,
    disposable_root: &std::ffi::OsStr,
    workspace_root: &std::ffi::OsStr,
    scenario: &std::ffi::OsStr,
    model: &std::ffi::OsStr,
) -> Result<(), HostExit> {
    use agentmage_host::coding_development_activation::CodingDevelopmentActivation;
    use agentmage_host::coding_development_runtime::{
        CodingDevelopmentModel, CodingDevelopmentRuntimeFactory, CodingDevelopmentScenario,
    };
    use agentmage_host::coding_live_runtime::LiveCodingRuntimeService;
    use agentmage_host::runtime_ipc::serve_linux_runtime_ipc;

    let scenario = scenario
        .to_str()
        .and_then(CodingDevelopmentScenario::parse)
        .ok_or(HostExit("coding.development.scenario-denied"))?;
    let model = model
        .to_str()
        .and_then(CodingDevelopmentModel::parse)
        .ok_or(HostExit("coding.development.model-denied"))?;
    let activation = CodingDevelopmentActivation::validate(
        std::path::Path::new(state_root),
        std::path::Path::new(disposable_root),
        std::path::Path::new(workspace_root),
    )
    .map_err(|error| HostExit(error.code()))?;
    let factory = CodingDevelopmentRuntimeFactory::new(activation.clone(), scenario, model)
        .map_err(|error| HostExit(error.code()))?;
    let parent = linux_bootstrap::parent_process_id().map_err(|error| HostExit(error.code()))?;
    let bootstrap = linux_bootstrap::bootstrap_development_for_peer(&activation, parent)
        .map_err(|error| HostExit(error.code()))?;
    bootstrap
        .write_launch_envelope(&mut std::io::stdout().lock())
        .map_err(|error| HostExit(error.code()))?;
    let mut session = bootstrap
        .accept()
        .map_err(|error| HostExit(error.kind().code()))?;
    let mut runtime = LiveCodingRuntimeService::new(factory);
    serve_linux_runtime_ipc(&mut session, &mut runtime).map_err(|error| HostExit(error.code()))
}

#[cfg(target_os = "linux")]
fn bootstrap_linux() -> Result<(), HostExit> {
    let runtime = linux_bootstrap::production_runtime_directory();
    let paths = linux_bootstrap::LinuxBootstrapPaths::new(
        std::ffi::OsStr::new(linux_bootstrap::PRODUCTION_PACKAGE_ROOT),
        std::ffi::OsStr::new(linux_bootstrap::PRODUCTION_PACKAGE_SIGNATURE),
        std::ffi::OsStr::new(linux_bootstrap::PRODUCTION_PACKAGE_PUBLIC_KEY),
        &runtime,
    );
    let parent = linux_bootstrap::parent_process_id().map_err(|error| HostExit(error.code()))?;
    let bootstrap = linux_bootstrap::bootstrap_for_peer(&paths, parent)
        .map_err(|error| HostExit(error.code()))?;
    let observed_at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .filter(|value| *value > 0)
        .ok_or(HostExit("agentmage.model-catalog.clock-unavailable"))?;
    let _model_picker = model_catalog_bootstrap::load_signed_model_picker_snapshot(
        bootstrap.verified_package(),
        observed_at_ms,
    )
    .map_err(|error| HostExit(error.code()))?;
    bootstrap
        .write_launch_envelope(&mut std::io::stdout().lock())
        .map_err(|error| HostExit(error.code()))?;
    let _session = bootstrap.accept().map_err(|error| HostExit(error.code()))?;
    Err(HostExit("agentmage.bootstrap.platform_activation_required"))
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
