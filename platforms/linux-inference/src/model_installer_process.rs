//! One-shot model installer process boundary.

use std::ffi::OsString;
use std::io::{Read, Write};

use agentmage_kernel_contracts::ExactModelProfile;
use serde::{Deserialize, Serialize};

use crate::{ModelAcquisitionHost, preflight_model_acquisition};

const MAX_PREFLIGHT_INPUT_BYTES: u64 = 1024 * 1024;

/// Exact inactive process descriptor emitted by the packaged installer.
pub const MODEL_INSTALLER_SELF_CHECK: &[u8] = b"{\"accepted_operations\":[\"preflight-stdin\",\"self-check\"],\"activation_authority\":false,\"component_id\":\"agentmage-model-installer\",\"inference_authority\":false,\"network_authority\":false,\"normal_operation\":false,\"one_shot\":true,\"protocol_version\":2,\"session_authority\":false,\"tool_authority\":false,\"workspace_authority\":false}\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelInstallerOperation {
    SelfCheck,
    PreflightStdin,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelPreflightRequest {
    schema_version: u16,
    profile: ExactModelProfile,
    host: ModelAcquisitionHost,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelPreflightResponse {
    schema_version: u16,
    operation: &'static str,
    preflight: crate::ModelAcquisitionPreflight,
}

/// Stable refusal from the inactive installer executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelInstallerProcessError {
    /// No lifecycle operation is admitted through the executable protocol yet.
    OperationUnavailable,
    /// Standard-input data was absent, malformed, oversized, or outside the closed schema.
    InputInvalid,
    /// The bounded response could not be serialized or written completely.
    OutputUnavailable,
}

impl ModelInstallerProcessError {
    /// Returns the stable content-free process error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::OperationUnavailable => "model.installer.operation-unavailable",
            Self::InputInvalid => "model.installer.input-invalid",
            Self::OutputUnavailable => "model.installer.output-unavailable",
        }
    }
}

impl std::fmt::Display for ModelInstallerProcessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ModelInstallerProcessError {}

/// Evaluates the exact installer argument closure without reading ambient state.
pub fn evaluate_model_installer_arguments(
    arguments: &[OsString],
) -> Result<&'static [u8], ModelInstallerProcessError> {
    match parse_operation(arguments)? {
        ModelInstallerOperation::SelfCheck => Ok(MODEL_INSTALLER_SELF_CHECK),
        ModelInstallerOperation::PreflightStdin => {
            Err(ModelInstallerProcessError::OperationUnavailable)
        }
    }
}

fn parse_operation(
    arguments: &[OsString],
) -> Result<ModelInstallerOperation, ModelInstallerProcessError> {
    match arguments {
        [command] if command == "--self-check" => Ok(ModelInstallerOperation::SelfCheck),
        [command] if command == "--preflight-stdin" => Ok(ModelInstallerOperation::PreflightStdin),
        _ => Err(ModelInstallerProcessError::OperationUnavailable),
    }
}

/// Executes one closed, one-shot installer operation without ambient authority.
pub fn execute_model_installer(
    arguments: &[OsString],
    mut input: impl Read,
    mut output: impl Write,
) -> Result<(), ModelInstallerProcessError> {
    match parse_operation(arguments)? {
        ModelInstallerOperation::SelfCheck => output
            .write_all(MODEL_INSTALLER_SELF_CHECK)
            .map_err(|_| ModelInstallerProcessError::OutputUnavailable),
        ModelInstallerOperation::PreflightStdin => {
            let mut bytes = Vec::new();
            input
                .by_ref()
                .take(MAX_PREFLIGHT_INPUT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| ModelInstallerProcessError::InputInvalid)?;
            if bytes.is_empty() || bytes.len() as u64 > MAX_PREFLIGHT_INPUT_BYTES {
                return Err(ModelInstallerProcessError::InputInvalid);
            }
            let request: ModelPreflightRequest = serde_json::from_slice(&bytes)
                .map_err(|_| ModelInstallerProcessError::InputInvalid)?;
            if request.schema_version != 1 {
                return Err(ModelInstallerProcessError::InputInvalid);
            }
            let response = ModelPreflightResponse {
                schema_version: 1,
                operation: "preflight",
                preflight: preflight_model_acquisition(&request.profile, &request.host),
            };
            serde_json::to_writer(&mut output, &response)
                .map_err(|_| ModelInstallerProcessError::OutputUnavailable)?;
            output
                .write_all(b"\n")
                .map_err(|_| ModelInstallerProcessError::OutputUnavailable)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use serde_json::{Value, json};

    use super::{
        MODEL_INSTALLER_SELF_CHECK, evaluate_model_installer_arguments, execute_model_installer,
    };

    #[test]
    fn only_exact_self_check_is_representable() {
        assert_eq!(
            evaluate_model_installer_arguments(&[OsString::from("--self-check")]),
            Ok(MODEL_INSTALLER_SELF_CHECK)
        );
        for arguments in [
            vec![],
            vec![OsString::from("--install")],
            vec![OsString::from("--model"), OsString::from("/tmp/model")],
            vec![OsString::from("--self-check"), OsString::from("extra")],
        ] {
            assert!(evaluate_model_installer_arguments(&arguments).is_err());
        }
    }

    #[test]
    fn exact_preflight_is_non_acquiring_closed_and_content_free() {
        let catalog: Value = serde_json::from_str(include_str!(
            "../../../model-profiles/exact-profile-catalog.json"
        ))
        .expect("catalog");
        let profile = catalog["profiles"]
            .as_array()
            .expect("profiles")
            .iter()
            .find(|value| {
                value["profile_id"]
                    == "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
            })
            .expect("profile")
            .clone();
        let envelope = profile["hardware"]
            .as_array()
            .expect("hardware")
            .iter()
            .find(|value| {
                value["platform"] == profile["runtime"]["platform"]
                    && value["architecture"] == profile["runtime"]["architecture"]
            })
            .expect("hardware envelope");
        let artifact_bytes = profile["artifact"]["bytes"]
            .as_u64()
            .expect("artifact bytes");
        let host = json!({
            "platform": profile["runtime"]["platform"],
            "architecture": profile["runtime"]["architecture"],
            "system_memory_bytes": envelope["minimum_system_memory_bytes"],
            "accelerator_memory_bytes": envelope["minimum_accelerator_memory_bytes"],
            "model_store_available_bytes": artifact_bytes.saturating_mul(2).saturating_add(1 << 30),
            "requested_context_tokens": 8_192,
            "runtime": profile["runtime"],
        });
        let request = json!({"schema_version": 1, "profile": profile, "host": host});
        let mut output = Vec::new();
        execute_model_installer(
            &[OsString::from("--preflight-stdin")],
            serde_json::to_vec(&request).expect("request").as_slice(),
            &mut output,
        )
        .expect("preflight");
        let response: Value = serde_json::from_slice(&output).expect("response");
        assert_eq!(response["schema_version"], 1);
        assert_eq!(response["operation"], "preflight");
        assert_eq!(response["preflight"]["disposition"], "eligible");
        assert_eq!(response["preflight"]["source_opened"], false);
        assert_eq!(response["preflight"]["destination_changed"], false);
        assert_eq!(
            response["preflight"]["review"]["license_spdx"],
            "Apache-2.0"
        );
    }

    #[test]
    fn malformed_oversized_and_effectful_operations_fail_closed() {
        for input in [b"".as_slice(), b"{}", b"{\"schema_version\":2}"] {
            assert!(
                execute_model_installer(&[OsString::from("--preflight-stdin")], input, Vec::new(),)
                    .is_err()
            );
        }
        let oversized = vec![b' '; super::MAX_PREFLIGHT_INPUT_BYTES as usize + 1];
        assert!(
            execute_model_installer(
                &[OsString::from("--preflight-stdin")],
                oversized.as_slice(),
                Vec::new(),
            )
            .is_err()
        );
        for arguments in [
            vec![OsString::from("--import")],
            vec![OsString::from("--download")],
            vec![OsString::from("--activate")],
            vec![OsString::from("--rollback")],
        ] {
            assert!(execute_model_installer(&arguments, b"".as_slice(), Vec::new()).is_err());
        }
    }
}
