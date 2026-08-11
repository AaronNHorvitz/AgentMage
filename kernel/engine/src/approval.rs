//! Deterministic construction and verification of non-authoritative approval displays.

use std::fmt::Write;

use agentmage_kernel_contracts::{ApprovalRequest, GrantTarget, to_canonical_json};
use sha2::{Digest, Sha256};

use crate::tooling::ToolRegistry;

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_PATH_COMPONENTS: usize = 64;
const MAX_PATH_COMPONENT_BYTES: usize = 255;
const MAX_TARGETS: usize = 128;
const MAX_EFFECTS: usize = 128;
const MAX_ROLLBACK_BYTES: usize = 1_024;
const MAX_GRANT_LIFETIME_MS: u64 = 86_400_000;
const ZERO_CONFIRMATION_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable reason an approval display cannot be rendered or verified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalRenderError {
    /// The shared approval shape or bounded scope is malformed.
    InvalidInput,
    /// The nested tool call is absent, malformed, or differs from its registered contract.
    InvalidToolCall,
    /// The retained confirmation digest does not match the exact current display object.
    ConfirmationMismatch,
}

impl ApprovalRenderError {
    /// Returns the stable redacted code used by interface diagnostics.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "approval.render.invalid_input",
            Self::InvalidToolCall => "approval.render.invalid_tool_call",
            Self::ConfirmationMismatch => "approval.verify.confirmation_mismatch",
        }
    }
}

/// Validates one candidate display and replaces its confirmation with a canonical digest.
///
/// The digest is SHA-256 over canonical `ApprovalRequest` JSON with `confirmation_sha256` set to
/// 64 lowercase zeroes. Rendering does not record a user decision, issue a grant, or authorize an
/// operation.
pub fn render_approval_request(
    registry: &ToolRegistry,
    mut candidate: ApprovalRequest,
) -> Result<ApprovalRequest, ApprovalRenderError> {
    candidate.confirmation_sha256 = ZERO_CONFIRMATION_SHA256.to_owned();
    validate_shape(registry, &candidate)?;
    candidate.confirmation_sha256 = confirmation_sha256(&candidate)?;
    Ok(candidate)
}

/// Verifies the exact shape, registered tool contract, and canonical confirmation digest.
///
/// A successful verification means only that the display object is internally consistent. The
/// object remains non-authoritative and cannot satisfy policy or dispatch boundaries.
pub fn verify_approval_request(
    registry: &ToolRegistry,
    request: &ApprovalRequest,
) -> Result<(), ApprovalRenderError> {
    validate_shape(registry, request)?;
    if !valid_digest(&request.confirmation_sha256)
        || confirmation_sha256(request)? != request.confirmation_sha256
    {
        return Err(ApprovalRenderError::ConfirmationMismatch);
    }
    Ok(())
}

fn validate_shape(
    registry: &ToolRegistry,
    request: &ApprovalRequest,
) -> Result<(), ApprovalRenderError> {
    if request.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION {
        return Err(ApprovalRenderError::InvalidInput);
    }
    for identifier in [
        request.approval_id.as_str(),
        request.proposed_grant_id.as_str(),
        request.parent_grant_id.as_str(),
        request.actor_id.as_str(),
        request.session_id.as_str(),
        request.task_id.as_str(),
    ] {
        validate_identifier(identifier)?;
    }
    if !valid_digest(&request.parent_grant_sha256) || !valid_digest(&request.policy_sha256) {
        return Err(ApprovalRenderError::InvalidInput);
    }
    if request.expires_at_epoch_ms <= request.issued_at_epoch_ms
        || request.expires_at_epoch_ms - request.issued_at_epoch_ms > MAX_GRANT_LIFETIME_MS
        || request.rollback_description.trim().is_empty()
        || request.rollback_description.len() > MAX_ROLLBACK_BYTES
    {
        return Err(ApprovalRenderError::InvalidInput);
    }
    validate_scope(&request.targets, &request.excluded_targets)?;
    validate_preimages_and_effects(request)?;
    let definition = registry
        .validate_arguments(&request.tool_call)
        .map_err(|_| ApprovalRenderError::InvalidToolCall)?;
    if definition.required_grant.operation != request.operation
        || definition.declared_effects.as_slice() != [request.operation]
    {
        return Err(ApprovalRenderError::InvalidToolCall);
    }
    Ok(())
}

fn validate_preimages_and_effects(request: &ApprovalRequest) -> Result<(), ApprovalRenderError> {
    if request.preimages.len() > MAX_TARGETS
        || request.expected_side_effects.is_empty()
        || request.expected_side_effects.len() > MAX_EFFECTS
    {
        return Err(ApprovalRenderError::InvalidInput);
    }
    let mut preimage_indexes = std::collections::BTreeSet::new();
    for preimage in &request.preimages {
        if !valid_target_index(preimage.target_index, request.targets.len())
            || !preimage_indexes.insert(preimage.target_index)
            || !valid_digest(&preimage.content_sha256)
            || preimage.observed_revision.as_ref().is_some_and(|value| {
                value.is_empty()
                    || value.len() > MAX_IDENTIFIER_BYTES
                    || value.chars().any(char::is_control)
            })
        {
            return Err(ApprovalRenderError::InvalidInput);
        }
    }
    for effect in &request.expected_side_effects {
        if effect.operation != request.operation
            || effect.target_indexes.is_empty()
            || effect
                .target_indexes
                .iter()
                .any(|index| !valid_target_index(*index, request.targets.len()))
            || !valid_digest(&effect.details_sha256)
        {
            return Err(ApprovalRenderError::InvalidInput);
        }
    }
    Ok(())
}

fn validate_scope(
    targets: &[GrantTarget],
    excluded_targets: &[GrantTarget],
) -> Result<(), ApprovalRenderError> {
    if targets.is_empty() || targets.len() > MAX_TARGETS || excluded_targets.len() > MAX_TARGETS {
        return Err(ApprovalRenderError::InvalidInput);
    }
    for target in targets.iter().chain(excluded_targets) {
        validate_identifier(target.workspace_id.as_str())?;
        if target.path_components.len() > MAX_PATH_COMPONENTS {
            return Err(ApprovalRenderError::InvalidInput);
        }
        for component in &target.path_components {
            if component.is_empty()
                || component.len() > MAX_PATH_COMPONENT_BYTES
                || matches!(component.as_str(), "." | ".." | "*" | "**")
                || component
                    .bytes()
                    .any(|value| matches!(value, 0 | b'/' | b'\\'))
            {
                return Err(ApprovalRenderError::InvalidInput);
            }
        }
    }
    let workspace = &targets[0].workspace_id;
    if targets
        .iter()
        .chain(excluded_targets)
        .any(|target| &target.workspace_id != workspace)
    {
        return Err(ApprovalRenderError::InvalidInput);
    }
    Ok(())
}

fn valid_target_index(index: u32, target_count: usize) -> bool {
    usize::try_from(index)
        .ok()
        .is_some_and(|value| value < target_count)
}

fn validate_identifier(value: &str) -> Result<(), ApprovalRenderError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || value.contains('*')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(ApprovalRenderError::InvalidInput);
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn confirmation_sha256(request: &ApprovalRequest) -> Result<String, ApprovalRenderError> {
    let mut preimage = request.clone();
    preimage.confirmation_sha256 = ZERO_CONFIRMATION_SHA256.to_owned();
    let bytes = to_canonical_json(&preimage).map_err(|_| ApprovalRenderError::InvalidInput)?;
    Ok(hex_sha256(&bytes))
}

fn hex_sha256(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{ApprovalRenderError, render_approval_request, verify_approval_request};
    use crate::{
        authority::{DescriptiveArtifactKind, reject_as_authority},
        tooling::{Tool, ToolRegistry},
    };
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, ApprovalId, ApprovalRequest, ContractPayload, CorrelationId,
        DataSensitivity, GrantId, GrantOperation, GrantPreimage, GrantSideEffect, GrantTarget,
        OperationBinding, RequiredGrantTemplate, SchemaId, SchemaReference, SessionId, TaskId,
        ToolCall, ToolCallId, ToolDefinition, ToolId, ToolRiskLevel, WorkspaceId,
    };

    struct FixtureTool {
        definition: ToolDefinition,
    }

    impl Tool for FixtureTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }
    }

    fn schema() -> SchemaReference {
        SchemaReference {
            schema_id: SchemaId::from_raw("fixture.input"),
            schema_version: 1,
            schema_sha256: "1".repeat(64),
        }
    }

    fn registry() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FixtureTool {
                definition: ToolDefinition {
                    schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                    tool_id: ToolId::from_raw("fixture.read"),
                    tool_version: "1.0.0".to_owned(),
                    display_name: "Fixture reader".to_owned(),
                    description: "Reads one synthetic fixture".to_owned(),
                    input_schema: schema(),
                    output_schema: schema(),
                    risk_level: ToolRiskLevel::Low,
                    declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
                    required_grant: RequiredGrantTemplate {
                        operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                        target_scope: "workspace-file".to_owned(),
                        single_use: true,
                    },
                    timeout_ms: 1_000,
                },
            }))
            .expect("fixture tool must register");
        registry
    }

    fn target(path: &[&str]) -> GrantTarget {
        GrantTarget {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            path_components: path.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn candidate() -> ApprovalRequest {
        let arguments = br#"{"path":["src","fixture.txt"]}"#.to_vec();
        ApprovalRequest {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            approval_id: ApprovalId::from_raw("approval-0001"),
            proposed_grant_id: GrantId::from_raw("grant-proposed-0001"),
            parent_grant_id: GrantId::from_raw("grant-parent-0001"),
            parent_grant_sha256: "2".repeat(64),
            actor_id: ActorId::from_raw("actor-local-0001"),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_kind: ActionKind::DeterministicTool,
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            tool_call: ToolCall {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_call_id: ToolCallId::from_raw("call-0001"),
                correlation_id: CorrelationId::from_raw("correlation-0001"),
                action_id: ActionId::from_raw("action-0001"),
                tool_id: ToolId::from_raw("fixture.read"),
                tool_version: "1.0.0".to_owned(),
                arguments: ContractPayload {
                    schema: schema(),
                    media_type: "application/json".to_owned(),
                    sha256: super::hex_sha256(&arguments),
                    bytes: arguments,
                },
            },
            targets: vec![target(&["src", "fixture.txt"])],
            excluded_targets: vec![target(&["src", "private"])],
            sensitivity: DataSensitivity::Ephemeral,
            preimages: vec![GrantPreimage {
                target_index: 0,
                content_sha256: "3".repeat(64),
                observed_revision: Some("fixture-v1".to_owned()),
            }],
            expected_side_effects: vec![GrantSideEffect {
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                target_indexes: vec![0],
                details_sha256: "4".repeat(64),
            }],
            rollback_description: "No state change is permitted".to_owned(),
            issued_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 30_000,
            policy_sha256: "5".repeat(64),
            confirmation_sha256: "untrusted-candidate-value".to_owned(),
        }
    }

    #[test]
    fn rendering_is_deterministic_exact_and_always_non_authoritative() {
        let registry = registry();
        let first = render_approval_request(&registry, candidate()).expect("approval must render");
        let second =
            render_approval_request(&registry, candidate()).expect("approval must render again");
        assert_eq!(first, second);
        assert_ne!(first.confirmation_sha256, "0".repeat(64));
        assert_eq!(verify_approval_request(&registry, &first), Ok(()));

        let denial = reject_as_authority(&first);
        assert_eq!(
            denial.artifact_kind,
            DescriptiveArtifactKind::ApprovalRequest
        );
        assert_eq!(denial.error.code, "authority.descriptive_artifact.denied");
    }

    #[test]
    fn every_exact_display_mutation_invalidates_confirmation() {
        let registry = registry();
        let rendered =
            render_approval_request(&registry, candidate()).expect("approval must render");
        let mutations: Vec<ApprovalRequest> = vec![
            {
                let mut value = rendered.clone();
                value.task_id = TaskId::from_raw("task-other");
                value
            },
            {
                let mut value = rendered.clone();
                value.targets[0].path_components.push("changed".to_owned());
                value
            },
            {
                let mut value = rendered.clone();
                value.preimages[0].content_sha256 = "a".repeat(64);
                value
            },
            {
                let mut value = rendered.clone();
                value.expected_side_effects[0].details_sha256 = "b".repeat(64);
                value
            },
            {
                let mut value = rendered.clone();
                value.expires_at_epoch_ms -= 1;
                value
            },
            {
                let mut value = rendered.clone();
                value.policy_sha256 = "c".repeat(64);
                value
            },
        ];
        for changed in mutations {
            assert_eq!(
                verify_approval_request(&registry, &changed),
                Err(ApprovalRenderError::ConfirmationMismatch)
            );
        }
    }

    #[test]
    fn malformed_call_wildcard_scope_and_mismatched_confirmation_fail_closed() {
        let registry = registry();
        let mut malformed_call = candidate();
        malformed_call.tool_call.arguments.bytes.push(b' ');
        assert_eq!(
            render_approval_request(&registry, malformed_call),
            Err(ApprovalRenderError::InvalidToolCall)
        );

        let mut wildcard = candidate();
        wildcard.targets[0].path_components = vec!["**".to_owned()];
        assert_eq!(
            render_approval_request(&registry, wildcard),
            Err(ApprovalRenderError::InvalidInput)
        );

        let mut mismatched =
            render_approval_request(&registry, candidate()).expect("approval must render");
        mismatched.confirmation_sha256 = "f".repeat(64);
        assert_eq!(
            verify_approval_request(&registry, &mismatched),
            Err(ApprovalRenderError::ConfirmationMismatch)
        );
    }
}
