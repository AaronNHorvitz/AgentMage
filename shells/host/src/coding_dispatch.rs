//! Authority-free preparation of exact native coding-tool calls.

use agentmage_capability_read_only::{
    GIT_INSPECTION_TOOL_ID, GIT_INSPECTION_TOOL_VERSION, GitCommandPlan, GitInspectionRequest,
    ReadOnlyRequest, ReadOnlyToolKind, plan_git_inspection, read_only_tool_kind,
    validate_git_inspection_request, validate_read_only_request,
};
use agentmage_kernel_contracts::ToolCall;
use agentmage_kernel_engine::{
    command_runner::{CommandRequest, PreparedCommand, prepare_command},
    tooling::ToolRegistry,
    validation_template::ValidationTemplate,
};

use crate::{
    coding_changes::{
        CONTROLLED_CHANGE_TOOL_VERSION, CONTROLLED_CREATE_TOOL_ID, ControlledFileCreationProposal,
        STRUCTURED_PATCH_TOOL_ID, StructuredPatchProposal,
    },
    coding_session::CodingSessionProfile,
    coding_tools::{
        BOUNDED_COMMAND_TOOL_ID, BOUNDED_COMMAND_TOOL_VERSION, TARGETED_VALIDATION_TOOL_ID,
        TARGETED_VALIDATION_TOOL_VERSION, TargetedValidationRequest,
    },
};

/// One validated, authority-free native operation ready for a trusted effect boundary.
#[derive(Clone, Debug)]
pub enum PreparedNativeCodingCall {
    /// A bounded workspace projection request.
    ReadOnly {
        /// Exact selected read-only operation.
        kind: ReadOnlyToolKind,
        /// Closed request validated for that operation.
        request: ReadOnlyRequest,
    },
    /// A fixed non-network Git inspection invocation.
    GitInspection {
        /// Closed Git inspection request.
        request: GitInspectionRequest,
        /// Exact shell-free argument, environment, and standard-input plan.
        plan: GitCommandPlan,
    },
    /// A structured edit proposal still requiring a trusted held preimage.
    StructuredPatch {
        /// Scope-validated inert model proposal.
        proposal: StructuredPatchProposal,
    },
    /// An absent-file proposal still requiring a trusted parent observation.
    ControlledCreate {
        /// Scope-validated inert model proposal.
        proposal: ControlledFileCreationProposal,
    },
    /// One exact frozen command template and deterministic preview.
    Command {
        /// Authority-free prepared command.
        prepared: Box<PreparedCommand>,
    },
    /// One exact validation template and its frozen command preview.
    Validation {
        /// Closed model selector for the validation attempt.
        request: TargetedValidationRequest,
        /// Exact trusted validation template selected by digest.
        template: Box<ValidationTemplate>,
        /// Authority-free prepared command owned by the template.
        prepared: Box<PreparedCommand>,
    },
}

/// Stable content-free refusal while preparing one native coding call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeCodingDispatchError {
    /// The call did not pass the exact immutable session registry.
    CallDenied,
    /// The registered provider could not reproduce its typed plan.
    ProviderDenied,
    /// The call selected no native coding provider in the closed catalog.
    ToolUnavailable,
}

impl NativeCodingDispatchError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::CallDenied => "runtime.coding-dispatch.call-denied",
            Self::ProviderDenied => "runtime.coding-dispatch.provider-denied",
            Self::ToolUnavailable => "runtime.coding-dispatch.tool-unavailable",
        }
    }
}

/// Prepares native calls against one immutable coding-session profile.
pub struct NativeCodingCallPreparer<'profile> {
    profile: &'profile CodingSessionProfile,
}

impl<'profile> NativeCodingCallPreparer<'profile> {
    /// Binds preparation to the exact registries frozen into one session profile.
    #[must_use]
    pub const fn new(profile: &'profile CodingSessionProfile) -> Self {
        Self { profile }
    }

    /// Validates and prepares one call without opening a path, launching a worker, or granting it.
    pub fn prepare(
        &self,
        call: &ToolCall,
    ) -> Result<PreparedNativeCodingCall, NativeCodingDispatchError> {
        prepare_from_parts(
            self.profile.registry(),
            self.profile.commands(),
            self.profile.validations(),
            call,
        )
    }
}

fn prepare_from_parts(
    registry: &ToolRegistry,
    commands: &agentmage_kernel_engine::command_runner::CommandRegistry,
    validations: &agentmage_kernel_engine::validation_template::ValidationTemplateRegistry,
    call: &ToolCall,
) -> Result<PreparedNativeCodingCall, NativeCodingDispatchError> {
    registry
        .validate_arguments(call)
        .map_err(|_| NativeCodingDispatchError::CallDenied)?;

    if let Some(kind) = read_only_tool_kind(&call.tool_id, &call.tool_version) {
        let request = validate_read_only_request(kind, &call.arguments.bytes)
            .map_err(|_| NativeCodingDispatchError::ProviderDenied)?;
        return Ok(PreparedNativeCodingCall::ReadOnly { kind, request });
    }

    match (call.tool_id.as_str(), call.tool_version.as_str()) {
        (GIT_INSPECTION_TOOL_ID, GIT_INSPECTION_TOOL_VERSION) => {
            let request = validate_git_inspection_request(&call.arguments.bytes)
                .map_err(|_| NativeCodingDispatchError::ProviderDenied)?;
            let plan = plan_git_inspection(&request)
                .map_err(|_| NativeCodingDispatchError::ProviderDenied)?;
            Ok(PreparedNativeCodingCall::GitInspection { request, plan })
        }
        (STRUCTURED_PATCH_TOOL_ID, CONTROLLED_CHANGE_TOOL_VERSION) => {
            let proposal = serde_json::from_slice(&call.arguments.bytes)
                .map_err(|_| NativeCodingDispatchError::ProviderDenied)?;
            Ok(PreparedNativeCodingCall::StructuredPatch { proposal })
        }
        (CONTROLLED_CREATE_TOOL_ID, CONTROLLED_CHANGE_TOOL_VERSION) => {
            let proposal = serde_json::from_slice(&call.arguments.bytes)
                .map_err(|_| NativeCodingDispatchError::ProviderDenied)?;
            Ok(PreparedNativeCodingCall::ControlledCreate { proposal })
        }
        (BOUNDED_COMMAND_TOOL_ID, BOUNDED_COMMAND_TOOL_VERSION) => {
            let request = serde_json::from_slice::<CommandRequest>(&call.arguments.bytes)
                .map_err(|_| NativeCodingDispatchError::ProviderDenied)?;
            let prepared = prepare_command(commands, request)
                .map_err(|_| NativeCodingDispatchError::ProviderDenied)?;
            Ok(PreparedNativeCodingCall::Command {
                prepared: Box::new(prepared),
            })
        }
        (TARGETED_VALIDATION_TOOL_ID, TARGETED_VALIDATION_TOOL_VERSION) => {
            let request =
                serde_json::from_slice::<TargetedValidationRequest>(&call.arguments.bytes)
                    .map_err(|_| NativeCodingDispatchError::ProviderDenied)?;
            let template = validations
                .resolve(&request.validation_id, &request.template_sha256)
                .cloned()
                .ok_or(NativeCodingDispatchError::ProviderDenied)?;
            let prepared = prepare_command(
                commands,
                CommandRequest::new(request.validation_attempt_id.clone(), &template.command),
            )
            .map_err(|_| NativeCodingDispatchError::ProviderDenied)?;
            Ok(PreparedNativeCodingCall::Validation {
                request,
                template: Box::new(template),
                prepared: Box::new(prepared),
            })
        }
        _ => Err(NativeCodingDispatchError::ToolUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fmt::Write;

    use agentmage_capability_read_only::{
        GitInspectionOperation, ReadOnlyEncoding, ReadOnlyLimits, read_only_tool_definition,
    };
    use agentmage_capability_repository_map::{
        StructuredArtifactClass, StructuredEdit, StructuredLanguage,
    };
    use agentmage_kernel_contracts::{
        ActionId, CONTRACT_SCHEMA_VERSION, ContractPayload, CorrelationId, ToolCallId, ToolId,
        WorkspaceId, WorkspacePath,
    };
    use agentmage_kernel_engine::{
        command_runner::{
            CommandBounds, CommandRegistry, CommandRisk, CommandSpec, CommandWorkingDirectory,
        },
        validation_template::{
            ValidationKind, ValidationParserKind, ValidationTemplateInput,
            ValidationTemplateRegistry, ValidationTemplateSource, seal_validation_template,
        },
    };
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::{
        coding_changes::{
            CodingWriteScope, ControlledFileClassification, controlled_create_tool_definition,
            structured_patch_tool_definition,
        },
        coding_tools::{
            bounded_command_tool_definition, native_coding_runtime_registry,
            targeted_validation_tool_definition,
        },
    };

    struct Fixture {
        registry: ToolRegistry,
        commands: CommandRegistry,
        validations: ValidationTemplateRegistry,
    }

    fn fixture() -> Fixture {
        let command = CommandSpec::seal(
            "fixture.check",
            "1.0.0",
            "/usr/bin/true",
            "a".repeat(64),
            Vec::new(),
            CommandWorkingDirectory::OwnedWorktree,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Low,
            CommandBounds::new(1_000, 1_024, 1_024, 32 * 1024 * 1024, 4, 100)
                .expect("command bounds"),
        )
        .expect("command");
        let commands = CommandRegistry::build(vec![command.clone()]).expect("commands");
        let validation = seal_validation_template(ValidationTemplateInput {
            validation_id: "validation-unit".to_owned(),
            kind: ValidationKind::Unit,
            command,
            source: ValidationTemplateSource::TrustedProjectConfiguration,
            source_sha256: "b".repeat(64),
            source_path: Some(
                WorkspacePath::new(WorkspaceId::from_raw("workspace-coding"), ["Cargo.toml"])
                    .expect("source path"),
            ),
            user_input_approval_sha256: None,
            execution_scope_sha256: "c".repeat(64),
            parser: ValidationParserKind::AgentMageJsonV1,
            parser_version: "1.0.0".to_owned(),
            parser_sha256: "d".repeat(64),
            minimum_test_count: 1,
            focused: true,
            fail_fast: true,
            failed_test_rerun_allowed: true,
            setup_idempotent: true,
            expected_artifacts: Vec::new(),
        })
        .expect("validation");
        let validations = ValidationTemplateRegistry::build(vec![validation]).expect("validations");
        let scope = CodingWriteScope::new(
            WorkspaceId::from_raw("workspace-coding"),
            vec![vec!["src".to_owned()], vec!["tests".to_owned()]],
        )
        .expect("scope");
        let registry = native_coding_runtime_registry(scope, commands.clone(), validations.clone())
            .expect("native registry");
        Fixture {
            registry,
            commands,
            validations,
        }
    }

    fn call(registry: &ToolRegistry, tool_id: &str, version: &str, bytes: Vec<u8>) -> ToolCall {
        let definition = registry
            .get_tool(&ToolId::from_raw(tool_id), version)
            .expect("registered definition");
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw(format!("call-{}", tool_id.replace('.', "-"))),
            correlation_id: CorrelationId::from_raw("correlation-coding-dispatch"),
            action_id: ActionId::from_raw("action-coding-dispatch"),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256_hex(&bytes),
                bytes,
            },
        }
    }

    fn read_request(kind: ReadOnlyToolKind) -> ReadOnlyRequest {
        let text = matches!(
            kind,
            ReadOnlyToolKind::ReadText
                | ReadOnlyToolKind::ReadMultiple
                | ReadOnlyToolKind::SearchText
        );
        let search = matches!(
            kind,
            ReadOnlyToolKind::SearchFilenames | ReadOnlyToolKind::SearchText
        );
        ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["src".to_owned()]],
            query: search.then(|| "runtime".to_owned()),
            byte_offset: None,
            byte_count: None,
            encoding: if text {
                ReadOnlyEncoding::Utf8
            } else {
                ReadOnlyEncoding::Binary
            },
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        }
    }

    fn prepare(
        fixture: &Fixture,
        call: &ToolCall,
    ) -> Result<PreparedNativeCodingCall, NativeCodingDispatchError> {
        prepare_from_parts(
            &fixture.registry,
            &fixture.commands,
            &fixture.validations,
            call,
        )
    }

    #[test]
    fn story_48_2_dispatches_every_read_provider_without_effect_authority() {
        let fixture = fixture();
        for kind in ReadOnlyToolKind::ALL {
            let definition = read_only_tool_definition(kind);
            let bytes = serde_json::to_vec(&read_request(kind)).expect("read request");
            let call = call(
                &fixture.registry,
                definition.tool_id.as_str(),
                &definition.tool_version,
                bytes,
            );
            let PreparedNativeCodingCall::ReadOnly {
                kind: prepared_kind,
                ..
            } = prepare(&fixture, &call).expect("prepared read")
            else {
                panic!("wrong provider");
            };
            assert_eq!(prepared_kind, kind);
        }
    }

    #[test]
    fn story_48_2_dispatches_git_patch_and_create_through_existing_contracts() {
        let fixture = fixture();
        let git = GitInspectionRequest {
            schema_version: 1,
            operation: GitInspectionOperation::Status,
            revision: None,
            object_id: None,
            pathspecs: Vec::new(),
            max_records: 100,
            max_output_bytes: 64 * 1024,
        };
        let git_call = call(
            &fixture.registry,
            GIT_INSPECTION_TOOL_ID,
            GIT_INSPECTION_TOOL_VERSION,
            serde_json::to_vec(&git).expect("git request"),
        );
        let PreparedNativeCodingCall::GitInspection { plan, .. } =
            prepare(&fixture, &git_call).expect("prepared Git")
        else {
            panic!("wrong Git provider");
        };
        assert_eq!(plan.argv[0], "git");
        assert!(plan.environment.contains_key("GIT_OPTIONAL_LOCKS"));

        let patch = StructuredPatchProposal {
            schema_version: 1,
            change_id: "change-0001".to_owned(),
            path: vec!["src".to_owned(), "guide.md".to_owned()],
            expected_preimage_sha256: sha256_hex(b"old\n"),
            intent_sha256: "e".repeat(64),
            change_plan_sha256: "f".repeat(64),
            language: StructuredLanguage::PlainText,
            artifact_class: StructuredArtifactClass::Documentation,
            edits: vec![StructuredEdit::ReplaceExactText {
                edit_id: "edit-0001".to_owned(),
                expected: "old".to_owned(),
                replacement: "new".to_owned(),
            }],
            additional_review_hooks: Vec::new(),
            generated: false,
            allow_generated: false,
        };
        let definition = structured_patch_tool_definition();
        let patch_call = call(
            &fixture.registry,
            definition.tool_id.as_str(),
            &definition.tool_version,
            serde_json::to_vec(&patch).expect("patch"),
        );
        assert!(matches!(
            prepare(&fixture, &patch_call).expect("prepared patch"),
            PreparedNativeCodingCall::StructuredPatch { .. }
        ));

        let create = ControlledFileCreationProposal {
            schema_version: 1,
            creation_id: "creation-0001".to_owned(),
            path: vec!["tests".to_owned(), "new.rs".to_owned()],
            content: "#[test]\nfn works() {}\n".to_owned(),
            mode: 0o644,
            classification: ControlledFileClassification::SourceCode,
            intent_sha256: "e".repeat(64),
            change_plan_sha256: "f".repeat(64),
            expected_parent_sha256: "1".repeat(64),
        };
        let definition = controlled_create_tool_definition();
        let create_call = call(
            &fixture.registry,
            definition.tool_id.as_str(),
            &definition.tool_version,
            serde_json::to_vec(&create).expect("create"),
        );
        assert!(matches!(
            prepare(&fixture, &create_call).expect("prepared create"),
            PreparedNativeCodingCall::ControlledCreate { .. }
        ));
    }

    #[test]
    fn story_48_2_dispatches_commands_and_validations_by_exact_digest() {
        let fixture = fixture();
        let command = fixture.commands.commands()[0];
        let definition = bounded_command_tool_definition();
        let command_call = call(
            &fixture.registry,
            definition.tool_id.as_str(),
            &definition.tool_version,
            serde_json::to_vec(&CommandRequest::new("command-attempt-0001", command))
                .expect("command request"),
        );
        let PreparedNativeCodingCall::Command { prepared } =
            prepare(&fixture, &command_call).expect("prepared command")
        else {
            panic!("wrong command provider");
        };
        assert_eq!(prepared.command(), command);

        let template = &fixture.validations.templates[0];
        let request = TargetedValidationRequest {
            schema_version: 1,
            validation_attempt_id: "validation-attempt-0001".to_owned(),
            validation_id: template.validation_id.clone(),
            template_sha256: template.template_sha256.clone(),
        };
        let definition = targeted_validation_tool_definition();
        let validation_call = call(
            &fixture.registry,
            definition.tool_id.as_str(),
            &definition.tool_version,
            serde_json::to_vec(&request).expect("validation request"),
        );
        let PreparedNativeCodingCall::Validation {
            template: prepared_template,
            prepared,
            ..
        } = prepare(&fixture, &validation_call).expect("prepared validation")
        else {
            panic!("wrong validation provider");
        };
        assert_eq!(*prepared_template, *template);
        assert_eq!(prepared.command(), &template.command);
    }

    #[test]
    fn story_48_2_dispatch_fails_closed_before_any_provider_plan_exists() {
        let fixture = fixture();
        let definition = read_only_tool_definition(ReadOnlyToolKind::ReadText);
        let mut malformed = call(
            &fixture.registry,
            definition.tool_id.as_str(),
            &definition.tool_version,
            b"{}".to_vec(),
        );
        assert!(matches!(
            prepare(&fixture, &malformed),
            Err(NativeCodingDispatchError::CallDenied)
        ));

        malformed.tool_version = "2.0.0".to_owned();
        assert!(matches!(
            prepare(&fixture, &malformed),
            Err(NativeCodingDispatchError::CallDenied)
        ));
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(64);
        for byte in Sha256::digest(bytes) {
            write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
        }
        output
    }
}
