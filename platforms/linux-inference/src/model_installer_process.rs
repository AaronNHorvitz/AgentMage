//! One-shot model installer process boundary.

use std::ffi::OsString;

/// Exact inactive process descriptor emitted by the packaged installer.
pub const MODEL_INSTALLER_SELF_CHECK: &[u8] = b"{\"accepted_operation\":\"self-check-only\",\"activation_authority\":false,\"component_id\":\"agentmage-model-installer\",\"inference_authority\":false,\"network_authority\":false,\"normal_operation\":false,\"one_shot\":true,\"protocol_version\":1,\"session_authority\":false,\"tool_authority\":false,\"workspace_authority\":false}\n";

/// Stable refusal from the inactive installer executable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelInstallerProcessError {
    /// No lifecycle operation is admitted through the executable protocol yet.
    OperationUnavailable,
}

impl ModelInstallerProcessError {
    /// Returns the stable content-free process error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::OperationUnavailable => "model.installer.operation-unavailable",
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
    match arguments {
        [command] if command == "--self-check" => Ok(MODEL_INSTALLER_SELF_CHECK),
        _ => Err(ModelInstallerProcessError::OperationUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{MODEL_INSTALLER_SELF_CHECK, evaluate_model_installer_arguments};

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
}
