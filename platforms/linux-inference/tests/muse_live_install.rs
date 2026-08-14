#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::time::Duration;

use agentmage_kernel_contracts::{ExactModelProfile, PlatformFamily, RuntimeIsolationObservation};
use agentmage_platform_linux_inference::{
    LinuxNativeModelAdapter, LlamaServerDriver, LlamaServerDriverConfig, ModelAcquisitionHost,
    ModelActivationDisposition, ModelImportDisposition, NativeModelInstallVerifier,
    activate_verified_model, import_local_model, preflight_model_acquisition,
};

fn required_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required")))
}

fn muse_profile() -> ExactModelProfile {
    let catalog: serde_json::Value = serde_json::from_str(include_str!(
        "../../../model-profiles/exact-profile-catalog.json"
    ))
    .expect("catalog");
    serde_json::from_value(
        catalog["profiles"]
            .as_array()
            .expect("profiles")
            .iter()
            .find(|value| {
                value["profile_id"]
                    == "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
            })
            .expect("Muse diagnostic profile")
            .clone(),
    )
    .expect("exact profile")
}

#[test]
#[ignore = "loads the exact 16.7 GB Muse artifact on pinned local hardware"]
fn exact_muse_import_scan_load_unload_and_evidence_activation() {
    let source = required_path("AGENTMAGE_MUSE_SOURCE");
    let store = required_path("AGENTMAGE_MUSE_STORE");
    let runtime_root = required_path("AGENTMAGE_MUSE_RUNTIME_ROOT");
    let socket_root = required_path("AGENTMAGE_MUSE_SOCKET_ROOT");
    let profile = muse_profile();
    let preflight = preflight_model_acquisition(
        &profile,
        &ModelAcquisitionHost {
            platform: PlatformFamily::Fedora,
            architecture: profile.runtime.architecture,
            system_memory_bytes: 64 * 1024 * 1024 * 1024,
            accelerator_memory_bytes: 24 * 1024 * 1024 * 1024,
            model_store_available_bytes: 64 * 1024 * 1024 * 1024,
            requested_context_tokens: 8192,
            runtime: Some(profile.runtime.clone()),
        },
    );
    assert!(preflight.blockers.is_empty(), "{:?}", preflight.blockers);
    let reuse_verified = std::env::var_os("AGENTMAGE_MUSE_REUSE_VERIFIED").as_deref()
        == Some(std::ffi::OsStr::new("1"));
    let (verified_name, copied_bytes) = if reuse_verified {
        let name = format!(".verified-import-{}.gguf", profile.artifact.sha256);
        assert!(
            store.join(&name).is_file(),
            "verified retry input is absent"
        );
        (name, 0)
    } else {
        let imported = import_local_model(&profile, &preflight, &source, &store, || false)
            .expect("exact local import");
        assert_eq!(imported.disposition, ModelImportDisposition::VerifiedStaged);
        (
            imported.retained_name.expect("verified staging name"),
            imported.copied_bytes,
        )
    };
    let verified_path = store.join(&verified_name);
    let socket_path = socket_root.join("llama-server.sock");
    let identity = profile.runtime.clone();
    let driver = LlamaServerDriver::new(
        LlamaServerDriverConfig::new(
            runtime_root,
            verified_path,
            socket_path,
            identity.clone(),
            Duration::from_secs(600),
        )
        .expect("exact driver configuration"),
    );
    let isolation = RuntimeIsolationObservation {
        adapter_id: identity.adapter_id.clone(),
        profile_id: profile.profile_id.clone(),
        network_available: false,
        workspace_available: false,
        authority_material_available: false,
        credential_material_available: false,
        observation_sha256: "d9c16abddc0740d234f768c8bea20c6d87680897292bb3d72fb4bdf846cb3236"
            .to_owned(),
    };
    let adapter = LinuxNativeModelAdapter::new(identity.clone(), isolation, driver)
        .expect("isolated native adapter");
    let mut verifier = NativeModelInstallVerifier::new(adapter, identity);
    let activated =
        activate_verified_model(&profile, &store, &verified_name, &mut verifier, |_| false)
            .expect("scan, load, unload, and evidence-store activation");
    assert_eq!(
        activated.disposition,
        ModelActivationDisposition::Activated,
        "native verifier failure: {:?}",
        verifier.last_failure_code()
    );
    assert_eq!(activated.active_profile_id, Some(profile.profile_id));
    assert!(!activated.workspace_available);
    assert!(!activated.session_available);
    assert!(!activated.unrelated_inference_available);
    assert!(!activated.tool_available);
    assert!(!activated.network_available);
    println!(
        "MUSE_LIVE_INSTALL_PASS generation={} copied_bytes={} retries=0",
        activated.generation, copied_bytes
    );
}
