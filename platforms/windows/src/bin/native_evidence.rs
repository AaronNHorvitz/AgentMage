//! Emits one redacted Windows-native identity evidence record.

#[cfg(target_os = "windows")]
fn main() -> Result<(), agentmage_platform_windows::WindowsNativeIdentityError> {
    let identity = agentmage_platform_windows::observe_windows_process_identity()?;
    println!(
        "{{\"schema_version\":1,\"record_type\":\"agentmage-windows-native-identity\",\"native_windows\":true,\"process_identity_observed\":{},\"session_identity_observed\":true,\"elevation_observed\":true,\"process_elevated\":{},\"user_sid_digest_nonzero\":{},\"executable_digest_nonzero\":{}}}",
        identity.process_id() > 0,
        identity.elevated(),
        identity.user_sid_sha256() != &[0; 32],
        identity.executable_sha256() != &[0; 32],
    );
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("agentmage.windows.native_evidence.requires_windows");
    std::process::exit(2);
}
