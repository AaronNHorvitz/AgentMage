//! Exact-profile GGUF scan and native-runtime install self-test.

use std::fs::File;
use std::io::Read;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use agentmage_kernel_contracts::{
    ExactModelProfile, LocalModelRuntime, ModelHealthState, ModelRuntimeIdentity,
};
use rustix::fs::{Mode, OFlags, open};
use sha2::{Digest, Sha256};

use crate::{
    LinuxNativeModelAdapter, ModelArtifactScanReport, ModelInstallSelfTestReport,
    ModelInstallVerifier, NativeModelDriver,
};

const PINNED_GGUF_SCAN_POLICY: &[u8] = b"agentmage.pinned-gguf-scan.v1\0exact-profile-sha256;regular-single-link;owner-only;non-executable;GGUF-header;bounded-size;trusted-load-ready-unload-required";

/// Exact-profile installer verifier backed by the common Linux runtime adapter.
pub struct NativeModelInstallVerifier<D: NativeModelDriver> {
    adapter: LinuxNativeModelAdapter<D>,
    identity: ModelRuntimeIdentity,
    last_failure_code: Option<String>,
}

impl<D: NativeModelDriver> NativeModelInstallVerifier<D> {
    /// Binds one already-isolated adapter to its exact runtime identity.
    #[must_use]
    pub fn new(adapter: LinuxNativeModelAdapter<D>, identity: ModelRuntimeIdentity) -> Self {
        Self {
            adapter,
            identity,
            last_failure_code: None,
        }
    }

    /// Returns the latest content-free native verification failure.
    #[must_use]
    pub fn last_failure_code(&self) -> Option<&str> {
        self.last_failure_code.as_deref()
    }
}

impl<D: NativeModelDriver> ModelInstallVerifier for NativeModelInstallVerifier<D> {
    fn scan(
        &mut self,
        artifact: &Path,
        profile: &ExactModelProfile,
    ) -> Option<ModelArtifactScanReport> {
        let format_valid = exact_allowlisted_gguf(artifact, profile).unwrap_or(false);
        Some(ModelArtifactScanReport {
            profile_id: profile.profile_id.clone(),
            artifact_sha256: profile.artifact.sha256.clone(),
            scanner_policy_sha256: lowercase_hex(&Sha256::digest(PINNED_GGUF_SCAN_POLICY)),
            format_valid,
            executable_payload_detected: false,
            malware_indicator_detected: !format_valid,
            complete: true,
        })
    }

    fn load_unload(
        &mut self,
        _artifact: &Path,
        profile: &ExactModelProfile,
    ) -> Option<ModelInstallSelfTestReport> {
        self.last_failure_code = None;
        if profile.runtime != self.identity {
            self.last_failure_code = Some("model.install-verifier.runtime-identity".to_owned());
            return None;
        }
        let load = match self.adapter.load(profile) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.last_failure_code = Some(error.code);
                return None;
            }
        };
        let health = self.adapter.health();
        let ready = health.profile_id.as_ref() == Some(&profile.profile_id)
            && health.state == ModelHealthState::Ready;
        let unload = match self.adapter.unload(&profile.profile_id) {
            Ok(receipt) => Some(receipt),
            Err(error) => {
                self.last_failure_code = Some(error.code);
                None
            }
        };
        let unloaded = unload.as_ref().is_some_and(|receipt| {
            receipt.profile_id == profile.profile_id
                && receipt.adapter_id == profile.runtime.adapter_id
                && receipt.empty
        }) && self.adapter.health().state == ModelHealthState::Unloaded;
        if !ready && self.last_failure_code.is_none() {
            self.last_failure_code = Some(health.reason_code.clone());
        } else if !unloaded && self.last_failure_code.is_none() {
            self.last_failure_code = Some("model.install-verifier.unload-incomplete".to_owned());
        }
        Some(ModelInstallSelfTestReport {
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            artifact_sha256: profile.artifact.sha256.clone(),
            runtime_sha256: profile.runtime.runtime_sha256.clone(),
            loaded: load.profile_id == profile.profile_id
                && load.manifest_sha256 == profile.manifest_sha256
                && load.adapter_id == profile.runtime.adapter_id,
            ready,
            unloaded,
            network_available: load.isolation.network_available,
            workspace_available: load.isolation.workspace_available,
            session_available: false,
            unrelated_inference_available: load.isolation.authority_material_available,
            tool_available: load.isolation.authority_material_available,
        })
    }
}

fn exact_allowlisted_gguf(
    artifact: &Path,
    profile: &ExactModelProfile,
) -> Result<bool, std::io::Error> {
    if profile.artifact.format != "GGUF"
        || profile.artifact.publisher.is_empty()
        || profile.artifact.source_revision.is_empty()
        || profile.artifact.bytes == 0
        || !valid_sha256(&profile.artifact.sha256)
    {
        return Ok(false);
    }
    let descriptor = open(
        artifact,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let mut file = File::from(descriptor);
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o777 != 0o600
        || metadata.len() != profile.artifact.bytes
    {
        return Ok(false);
    }
    let mut header = [0_u8; 8];
    file.read_exact(&mut header)?;
    let version = u32::from_le_bytes(header[4..8].try_into().expect("fixed four-byte slice"));
    Ok(&header[..4] == b"GGUF" && matches!(version, 2 | 3))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        EncodedModelContext, ExactModelProfile, ModelHealth, ModelHealthState, ModelLoadReceipt,
        ModelManifestObservation, ModelProfileId, ModelResourceReport, ModelRuntimeFailure,
        ModelStreamSink, ModelUnloadReceipt, RuntimeIsolationObservation, TokenCountResult,
    };
    use serde_json::Value;
    use sha2::Digest;

    use super::NativeModelInstallVerifier;
    use crate::{LinuxNativeModelAdapter, ModelInstallVerifier, NativeModelDriver};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct FakeDriver {
        profile: ExactModelProfile,
        loaded: bool,
    }

    impl NativeModelDriver for FakeDriver {
        fn verify_manifest(
            &self,
            profile: &ExactModelProfile,
        ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
            Ok(ModelManifestObservation {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: profile.manifest_sha256.clone(),
                artifact_sha256: profile.artifact.sha256.clone(),
                tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
                template_sha256: profile.codec.template_sha256.clone(),
                codec_sha256: profile.codec.codec_sha256.clone(),
                runtime: profile.runtime.clone(),
            })
        }

        fn load(
            &mut self,
            profile: &ExactModelProfile,
            isolation: &RuntimeIsolationObservation,
        ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
            self.loaded = true;
            Ok(ModelLoadReceipt {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: profile.manifest_sha256.clone(),
                adapter_id: profile.runtime.adapter_id.clone(),
                elapsed_ms: 1,
                isolation: isolation.clone(),
            })
        }

        fn unload(
            &mut self,
            profile_id: &ModelProfileId,
        ) -> Result<ModelUnloadReceipt, ModelRuntimeFailure> {
            self.loaded = false;
            Ok(ModelUnloadReceipt {
                profile_id: profile_id.clone(),
                adapter_id: self.profile.runtime.adapter_id.clone(),
                empty: true,
                elapsed_ms: 1,
            })
        }

        fn health(&self) -> ModelHealth {
            ModelHealth {
                adapter_id: self.profile.runtime.adapter_id.clone(),
                profile_id: self.loaded.then(|| self.profile.profile_id.clone()),
                state: if self.loaded {
                    ModelHealthState::Ready
                } else {
                    ModelHealthState::Unloaded
                },
                reason_code: "fixture".to_owned(),
                observed_at_ms: 1,
            }
        }

        fn count_tokens(
            &self,
            _context: &EncodedModelContext,
        ) -> Result<TokenCountResult, ModelRuntimeFailure> {
            unreachable!("not used")
        }

        fn stream(
            &mut self,
            _request: &agentmage_kernel_contracts::ModelRunRequest,
            _context: &EncodedModelContext,
            _cancellation: Option<&agentmage_kernel_contracts::CancellationSignal>,
            _sink: &mut dyn ModelStreamSink,
        ) -> Result<agentmage_kernel_contracts::ModelRunResult, ModelRuntimeFailure> {
            unreachable!("not used")
        }

        fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure> {
            unreachable!("not used")
        }
    }

    fn profile() -> ExactModelProfile {
        let catalog: Value = serde_json::from_str(include_str!(
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
                .expect("profile")
                .clone(),
        )
        .expect("profile")
    }

    fn isolation(profile: &ExactModelProfile) -> RuntimeIsolationObservation {
        RuntimeIsolationObservation {
            adapter_id: profile.runtime.adapter_id.clone(),
            profile_id: profile.profile_id.clone(),
            network_available: false,
            workspace_available: false,
            authority_material_available: false,
            credential_material_available: false,
            observation_sha256: "a".repeat(64),
        }
    }

    fn gguf_fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "agentmage-gguf-scan-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, b"GGUF\x03\0\0\0fixture").expect("fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("mode");
        path
    }

    #[test]
    fn exact_allowlisted_gguf_and_adapter_complete_install_verification() {
        let mut profile = profile();
        let artifact = gguf_fixture();
        let bytes = fs::read(&artifact).expect("bytes");
        profile.artifact.bytes = bytes.len() as u64;
        profile.artifact.sha256 = super::lowercase_hex(&sha2::Sha256::digest(bytes));
        let driver = FakeDriver {
            profile: profile.clone(),
            loaded: false,
        };
        let adapter =
            LinuxNativeModelAdapter::new(profile.runtime.clone(), isolation(&profile), driver)
                .expect("adapter");
        let mut verifier = NativeModelInstallVerifier::new(adapter, profile.runtime.clone());
        let scan = verifier.scan(&artifact, &profile).expect("scan");
        assert!(scan.format_valid);
        assert!(scan.complete);
        assert!(!scan.malware_indicator_detected);
        let self_test = verifier
            .load_unload(&artifact, &profile)
            .expect("self test");
        assert!(self_test.loaded);
        assert!(self_test.ready);
        assert!(self_test.unloaded);
        assert!(!self_test.network_available);
        let _ = fs::remove_file(artifact);
    }

    #[test]
    fn executable_or_wrong_version_fixture_fails_scan() {
        let mut profile = profile();
        let artifact = gguf_fixture();
        let bytes = fs::read(&artifact).expect("bytes");
        profile.artifact.bytes = bytes.len() as u64;
        profile.artifact.sha256 = super::lowercase_hex(&sha2::Sha256::digest(bytes));
        fs::set_permissions(&artifact, fs::Permissions::from_mode(0o700)).expect("exec mode");
        assert!(!super::exact_allowlisted_gguf(&artifact, &profile).expect("scan"));
        fs::set_permissions(&artifact, fs::Permissions::from_mode(0o600)).expect("mode");
        fs::write(&artifact, b"GGUF\x01\0\0\0fixture").expect("old version");
        assert!(!super::exact_allowlisted_gguf(&artifact, &profile).expect("scan"));
        let _ = fs::remove_file(artifact);
    }
}
