#![forbid(unsafe_code)]

fn main() {
    let shared_components = (
        agentmage_kernel_contracts::COMPONENT_ID,
        agentmage_kernel_engine::COMPONENT_ID,
        agentmage_capability_read_only::COMPONENT_ID,
    );

    #[cfg(target_os = "linux")]
    let platform_component = agentmage_platform_linux::COMPONENT_ID;
    #[cfg(not(target_os = "linux"))]
    let platform_component = "platform-adapter-not-linked";

    let _composition = (shared_components, platform_component);
}

#[cfg(test)]
mod tests {
    #[test]
    fn host_scaffold_has_no_runtime_behavior() {
        assert_eq!(agentmage_kernel_engine::COMPONENT_ID, "kernel-engine");
    }
}
