#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use agentmage_kernel_contracts::{
    BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
    ContextPacketId, ContractPayload, CorrelationId, ExactModelProfile, LocalModelRuntime,
    ModelCancellationProbe, ModelContextPacket, ModelFamilyCodec, ModelMessage, ModelMessageId,
    ModelMessageRole, ModelRunId, ModelRunRequest, ModelRunTerminalState, ModelRuntimeFailure,
    ModelStreamSink, PlatformFamily, RuntimeIsolationObservation, SchemaId, SchemaReference,
    SessionId, StreamedModelFragment, TaskId, ToolCatalogId,
};
use agentmage_platform_linux_inference::{
    LinuxNativeModelAdapter, LlamaServerDriver, LlamaServerDriverConfig, ModelAcquisitionHost,
    ModelActivationDisposition, ModelImportDisposition, MuseAtemFamilyCodec, NativeModelDriver,
    NativeModelInstallVerifier, activate_verified_model, import_local_model,
    preflight_model_acquisition,
};
use sha2::{Digest, Sha256};

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

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn isolated_adapter(profile: &ExactModelProfile) -> LinuxNativeModelAdapter<LlamaServerDriver> {
    let store = required_path("AGENTMAGE_MUSE_STORE");
    let runtime_root = required_path("AGENTMAGE_MUSE_RUNTIME_ROOT");
    let socket_root = required_path("AGENTMAGE_MUSE_SOCKET_ROOT");
    let driver = LlamaServerDriver::new(
        LlamaServerDriverConfig::new(
            runtime_root,
            store.join(format!("{}.gguf", profile.artifact.sha256)),
            socket_root.join("llama-server.sock"),
            profile.runtime.clone(),
            Duration::from_secs(600),
        )
        .expect("exact driver configuration"),
    );
    LinuxNativeModelAdapter::new(
        profile.runtime.clone(),
        RuntimeIsolationObservation {
            adapter_id: profile.runtime.adapter_id.clone(),
            profile_id: profile.profile_id.clone(),
            network_available: false,
            workspace_available: false,
            authority_material_available: false,
            credential_material_available: false,
            observation_sha256: "d9c16abddc0740d234f768c8bea20c6d87680897292bb3d72fb4bdf846cb3236"
                .to_owned(),
        },
        driver,
    )
    .expect("isolated native adapter")
}

#[derive(Default)]
struct FragmentCapture {
    fragments: Vec<StreamedModelFragment>,
}

impl ModelStreamSink for FragmentCapture {
    fn accept(&mut self, fragment: StreamedModelFragment) -> Result<(), ModelRuntimeFailure> {
        self.fragments.push(fragment);
        Ok(())
    }
}

struct FragmentAwareCancellation {
    output_seen: Arc<AtomicBool>,
    signal: CancellationSignal,
}

impl ModelCancellationProbe for FragmentAwareCancellation {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        Ok(self
            .output_seen
            .load(Ordering::SeqCst)
            .then(|| self.signal.clone()))
    }
}

struct CancellingFragmentCapture {
    fragments: Vec<StreamedModelFragment>,
    output_seen: Arc<AtomicBool>,
}

impl ModelStreamSink for CancellingFragmentCapture {
    fn accept(&mut self, fragment: StreamedModelFragment) -> Result<(), ModelRuntimeFailure> {
        if !fragment.terminal && !fragment.bytes.is_empty() {
            self.output_seen.store(true, Ordering::SeqCst);
        }
        self.fragments.push(fragment);
        Ok(())
    }
}

fn packet(profile: &ExactModelProfile) -> ModelContextPacket {
    let bytes = b"Using only this synthetic sentence, give one short advisory response. Do not claim any tool use, file access, network access, or completed action.".to_vec();
    let input_bytes = u64::try_from(bytes.len()).expect("bounded fixture length");
    ModelContextPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        context_packet_id: ContextPacketId::from_raw("muse-live-context-1"),
        session_id: SessionId::from_raw("muse-live-session-1"),
        task_id: TaskId::from_raw("muse-live-task-1"),
        profile_id: profile.profile_id.clone(),
        manifest_sha256: profile.manifest_sha256.clone(),
        tool_catalog_id: ToolCatalogId::from_raw("muse-live-no-tools"),
        messages: vec![ModelMessage {
            message_id: ModelMessageId::from_raw("muse-live-message-1"),
            role: ModelMessageRole::User,
            content: ContractPayload {
                schema: SchemaReference {
                    schema_id: SchemaId::from_raw("muse-live-synthetic-text-v1"),
                    schema_version: 1,
                    schema_sha256:
                        "65304556f98640b383963b94453090691c96daebca6d98b0374ced7c09ff05fd"
                            .to_owned(),
                },
                media_type: "text/plain".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            },
        }],
        input_bytes,
        input_tokens: 1,
        packet_sha256: "49450cdbf37a24f5b78d6135b145a9157c7bf0d177d77a93b75c4ea02a39a280"
            .to_owned(),
    }
}

fn request(profile: &ExactModelProfile, run: &str) -> ModelRunRequest {
    ModelRunRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        model_run_id: ModelRunId::from_raw(run),
        correlation_id: CorrelationId::from_raw("muse-live-correlation-1"),
        context_packet_id: ContextPacketId::from_raw("muse-live-context-1"),
        profile_id: profile.profile_id.clone(),
        manifest_sha256: profile.manifest_sha256.clone(),
        adapter_id: profile.runtime.adapter_id.clone(),
        decoding_profile_id: profile.decoding.profile_id.clone(),
        max_output_tokens: 64,
        timeout_ms: 120_000,
    }
}

#[test]
#[ignore = "hashes the exact 16.7 GB Muse artifact through the native driver"]
fn exact_muse_driver_manifest_diagnostic() {
    let store = required_path("AGENTMAGE_MUSE_STORE");
    let runtime_root = required_path("AGENTMAGE_MUSE_RUNTIME_ROOT");
    let socket_root = required_path("AGENTMAGE_MUSE_SOCKET_ROOT");
    let profile = muse_profile();
    let verified_path = store.join(format!(".verified-import-{}.gguf", profile.artifact.sha256));
    let driver = LlamaServerDriver::new(
        LlamaServerDriverConfig::new(
            runtime_root,
            verified_path,
            socket_root.join("llama-server.sock"),
            profile.runtime.clone(),
            Duration::from_secs(600),
        )
        .expect("exact driver configuration"),
    );
    match driver.verify_manifest(&profile) {
        Ok(observation) => {
            assert_eq!(observation.profile_id, profile.profile_id);
            println!("MUSE_DRIVER_MANIFEST_PASS");
        }
        Err(error) => panic!("native driver manifest failure: {}", error.code),
    }
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

#[test]
#[ignore = "loads and runs the exact Muse artifact inside the native sandbox"]
fn exact_muse_sandboxed_advisory_cancellation_and_unload() {
    let profile = muse_profile();
    let codec = MuseAtemFamilyCodec::new(profile.codec.clone()).expect("exact Muse codec");
    let mut adapter = isolated_adapter(&profile);
    let load = adapter.load(&profile).expect("sandboxed exact load");
    assert!(!load.isolation.network_available);
    assert!(!load.isolation.workspace_available);
    assert!(!load.isolation.authority_material_available);
    assert!(!load.isolation.credential_material_available);
    assert_eq!(
        adapter.health().profile_id,
        Some(profile.profile_id.clone())
    );

    let packet = packet(&profile);
    let encoded = codec
        .encode_context(&profile, &packet)
        .expect("bounded synthetic context");
    let count = adapter
        .count_tokens(&encoded)
        .expect("exact tokenizer count");
    assert!(count.tokens > 0 && count.tokens <= 8192);

    let run_request = request(&profile, "muse-live-run-1");
    let mut capture = FragmentCapture::default();
    let result = adapter
        .stream(&run_request, &encoded, None, &mut capture)
        .expect("bounded exact inference");
    assert_eq!(result.fragment_count as usize, capture.fragments.len());
    assert!(result.fragment_count > 1);
    assert!(
        capture
            .fragments
            .last()
            .is_some_and(|fragment| fragment.terminal)
    );
    let response = capture
        .fragments
        .iter()
        .flat_map(|fragment| fragment.bytes.iter().copied())
        .collect::<Vec<_>>();
    assert_eq!(sha256(&response), result.response_sha256);
    match result.terminal_state {
        ModelRunTerminalState::Proposed => {
            let _decoded = codec
                .decode_proposal(&profile, &run_request, &response)
                .expect("closed exact proposal");
            assert!(result.proposal.is_none());
            assert!(result.failure.is_none());
        }
        ModelRunTerminalState::AdvisoryText => {
            assert!(result.proposal.is_none());
            assert!(result.failure.is_none());
        }
        ModelRunTerminalState::Rejected => {
            assert!(result.proposal.is_none());
            assert!(result.failure.is_some());
        }
        state => panic!("unexpected live inference terminal state: {state:?}"),
    }

    let mid_run_request = request(&profile, "muse-live-run-mid-cancelled");
    let output_seen = Arc::new(AtomicBool::new(false));
    let mid_run_cancellation = FragmentAwareCancellation {
        output_seen: Arc::clone(&output_seen),
        signal: CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("muse-live-mid-cancellation-1"),
            correlation_id: mid_run_request.correlation_id.clone(),
            task_id: packet.task_id.clone(),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Kernel,
        },
    };
    let mut mid_run_capture = CancellingFragmentCapture {
        fragments: Vec::new(),
        output_seen,
    };
    let mid_run_cancelled = adapter
        .stream(
            &mid_run_request,
            &encoded,
            Some(&mid_run_cancellation),
            &mut mid_run_capture,
        )
        .expect("mid-generation cancellation");
    assert_eq!(
        mid_run_cancelled.terminal_state,
        ModelRunTerminalState::Cancelled
    );
    assert!(mid_run_capture.fragments.len() >= 2);
    assert!(
        mid_run_capture
            .fragments
            .iter()
            .any(|fragment| { !fragment.terminal && !fragment.bytes.is_empty() })
    );
    assert!(
        mid_run_capture
            .fragments
            .last()
            .is_some_and(|fragment| { fragment.terminal && fragment.bytes.is_empty() })
    );
    let mid_run_response = mid_run_capture
        .fragments
        .iter()
        .flat_map(|fragment| fragment.bytes.iter().copied())
        .collect::<Vec<_>>();
    assert_eq!(sha256(&mid_run_response), mid_run_cancelled.response_sha256);
    assert!(mid_run_cancelled.resources.output_tokens > 0);
    assert!(mid_run_cancelled.proposal.is_none());
    assert!(mid_run_cancelled.failure.is_some());

    let cancel_request = request(&profile, "muse-live-run-cancelled");
    let cancellation = CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("muse-live-cancellation-1"),
        correlation_id: cancel_request.correlation_id.clone(),
        task_id: packet.task_id.clone(),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Kernel,
    };
    let mut cancelled_capture = FragmentCapture::default();
    let cancelled = adapter
        .stream(
            &cancel_request,
            &encoded,
            Some(&cancellation),
            &mut cancelled_capture,
        )
        .expect("pre-request cancellation");
    assert_eq!(cancelled.terminal_state, ModelRunTerminalState::Cancelled);
    assert_eq!(cancelled.resources.output_tokens, 0);
    assert!(cancelled.proposal.is_none());
    assert!(cancelled.failure.is_some());

    let resources = adapter.resources().expect("content-free resources");
    assert!(resources.resident_memory_bytes > 0);
    assert!(resources.accelerator_memory_bytes > 0);
    let unload = adapter.unload(&profile.profile_id).expect("bounded unload");
    assert!(unload.empty);
    assert_eq!(
        adapter.health().state,
        agentmage_kernel_contracts::ModelHealthState::Unloaded
    );
    assert!(
        !required_path("AGENTMAGE_MUSE_SOCKET_ROOT")
            .join("llama-server.sock")
            .exists()
    );
    println!(
        "MUSE_SANDBOX_INFERENCE_PASS terminal={:?} response_sha256={} input_tokens={} output_tokens={} fragments={} mid_cancel_fragments={} mid_cancel_tokens={} load_ms={} run_ms={} unload_ms={}",
        result.terminal_state,
        result.response_sha256,
        count.tokens,
        result.resources.output_tokens,
        result.fragment_count,
        mid_run_cancelled.fragment_count,
        mid_run_cancelled.resources.output_tokens,
        load.elapsed_ms,
        result.resources.elapsed_ms,
        unload.elapsed_ms,
    );
}
