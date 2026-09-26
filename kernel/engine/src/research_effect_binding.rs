//! Exact public-GET restrictions applied to an already consumed effect permit.
//!
//! This is not an authority issuer, network executor, durable budget reservation or
//! retrieval receipt. The existing coordinator and native effect owner retain those
//! responsibilities. A successful check does not by itself establish worker admission.

use std::fmt;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, GrantTarget, HeldWorkspaceRoot, ToolCall,
    ToolDefinition, to_canonical_json,
};
use sha2::{Digest, Sha256};

use crate::authority_transaction::EffectAuthorization;
use crate::research_fetch::{PreparedPublicGet, PublicGetDraft, PublicGetWorkerPacket};

const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
const MAX_CALL_BYTES: usize = 128 * 1024;

/// Content-free binding refusal. No rejected arguments or destination are logged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchEffectBindingError {
    /// The registered tool, call envelope or exact request bytes do not match.
    Call,
    /// The consumed permit names another effect, task, workspace or disclosure.
    Authority,
    /// The trusted current time is outside the original prepared interval.
    Deadline,
}

/// Immutable trusted-owner preparation, never a grant or worker launch permit.
pub struct PublicGetEffectBinding {
    call: ToolCall,
    root: GrantTarget,
    prepared: PreparedPublicGet,
    network_scope: String,
}

impl fmt::Debug for PublicGetEffectBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PublicGetEffectBinding")
            .field("request_sha256", &self.prepared.packet().sha256())
            .finish_non_exhaustive()
    }
}

impl PublicGetEffectBinding {
    /// Freezes the exact call and held root under an independently registered tool.
    /// The caller must be trusted task composition, not a model or retrieved page.
    /// This performs no effect and does not persist or consume a budget reservation.
    pub fn prepare(
        definition: &ToolDefinition,
        call: ToolCall,
        root: &impl HeldWorkspaceRoot,
        prepared: PreparedPublicGet,
    ) -> Result<Self, ResearchEffectBindingError> {
        let operation =
            agentmage_kernel_contracts::OperationBinding::new(GrantOperation::NetworkAccess);
        if call.schema_version != CONTRACT_SCHEMA_VERSION
            || definition.schema_version != CONTRACT_SCHEMA_VERSION
            || definition.tool_id != call.tool_id
            || definition.tool_version != call.tool_version
            || definition.input_schema != call.arguments.schema
            || definition.input_schema.schema_version != 1
            || definition.declared_effects != [operation]
            || definition.required_grant.operation != operation
            || !definition.required_grant.single_use
            || call.arguments.media_type != "application/json"
            || call.arguments.bytes.is_empty()
            || call.arguments.bytes.len() > MAX_ARGUMENT_BYTES
            || digest(&call.arguments.bytes) != call.arguments.sha256
            || call.tool_call_id.as_str() != prepared.packet().request().operation_id
            || definition.timeout_ms < prepared.packet().request().timeout_ms
            || [
                call.tool_call_id.as_str(),
                call.correlation_id.as_str(),
                call.action_id.as_str(),
                call.tool_id.as_str(),
                call.arguments.schema.schema_id.as_str(),
            ]
            .iter()
            .any(|id| id.is_empty() || id.len() > 128)
            || call.tool_version.is_empty()
            || call.tool_version.len() > 32
            || call.arguments.schema.schema_sha256.len() != 64
        {
            return Err(ResearchEffectBindingError::Call);
        }
        let draft: PublicGetDraft = serde_json::from_slice(&call.arguments.bytes)
            .map_err(|_| ResearchEffectBindingError::Call)?;
        // Preserve the original approved native JSON, like the existing registered
        // command wrapper. Struct field order must not replace that exact identity.
        // Closed decoding establishes semantic equality to the independently prepared
        // packet; full ToolCall equality below still binds every original byte.
        if &draft != prepared.packet().request()
            || to_canonical_json(&call)
                .map_err(|_| ResearchEffectBindingError::Call)?
                .len()
                > MAX_CALL_BYTES
        {
            return Err(ResearchEffectBindingError::Call);
        }
        let root = GrantTarget::held_workspace_root(root)
            .map_err(|_| ResearchEffectBindingError::Authority)?;
        let network_scope = format!("https:{}:443", draft.target.domain);
        Ok(Self {
            call,
            root,
            prepared,
            network_scope,
        })
    }

    /// Exact parent-prepared bytes, not evidence of admission or authority.
    #[must_use]
    pub const fn packet(&self) -> &PublicGetWorkerPacket {
        self.prepared.packet()
    }

    /// Checks a permit issued ONLY by the existing authority transaction owner.
    /// The native driver must additionally verify the held root's live identity,
    /// durable budget reservation, worker/runtime confinement and owned cleanup.
    /// Repeatedly calling this predicate cannot issue or consume another grant.
    pub fn validate(
        &self,
        authorization: &EffectAuthorization<'_>,
        root: &impl HeldWorkspaceRoot,
        now_epoch_ms: u64,
    ) -> Result<(), ResearchEffectBindingError> {
        let packet = self.packet();
        if now_epoch_ms < packet.prepared_at_epoch_ms()
            || now_epoch_ms >= packet.deadline_epoch_ms()
            || !authorization.grant_live_at(now_epoch_ms)
            || packet.deadline_epoch_ms() > authorization.grant_expires_at_epoch_ms()
        {
            return Err(ResearchEffectBindingError::Deadline);
        }
        let effects = authorization.expected_side_effects();
        if authorization.operation().operation() != GrantOperation::NetworkAccess
            || authorization.task_id().as_str() != packet.task_id()
            || authorization.call() != &self.call
            || authorization.network_scope() != Some(self.network_scope.as_str())
            || authorization.targets() != std::slice::from_ref(&self.root)
            || !authorization.authorizes_held_workspace_root(root)
            || effects.len() != 1
            || effects[0].operation != authorization.operation()
            || effects[0].target_indexes != [0]
            || effects[0].details_sha256 != packet.sha256()
        {
            return Err(ResearchEffectBindingError::Authority);
        }
        Ok(())
    }
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        authority_transaction::{
            AuthorityTransactionCoordinator, AuthorityTransactionRequest, EffectDriver,
            EffectLaunch, EffectResult,
        },
        grants::{DerivedOperationGrantRequest, GrantIssuer, SessionReadGrantRequest},
        policy::{
            PolicyDocument, PolicyEngine, PolicyEvaluationContext, ScopeRules, ToolPolicyBinding,
        },
        research_budget::{ResearchDepth, ResearchLimits, ResearchNetworkMode, ResearchScope},
        research_fetch::PublicGetTarget,
        test_target::scope,
        tooling::{Tool, ToolRegistry},
    };
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, AdapterInstanceId, ApprovalId, AuthorityTransactionId,
        AuthorizedWorkspaceHandle, ContractPayload, CorrelationId, DataSensitivity, GrantId,
        GrantNonce, GrantSideEffect, OperationAttemptId, OperationBinding, OperationOutcome,
        PathPlatform, RequiredGrantTemplate, SchemaId, SchemaReference, SessionId, StateChange,
        TaskId, ToolCallId, ToolId, ToolRiskLevel, WorkspaceAuthorizationId, WorkspaceId,
        WorkspaceObjectIdentity,
    };
    use std::collections::BTreeSet;

    #[derive(Debug)]
    struct Root {
        workspace: WorkspaceId,
        authorization: WorkspaceAuthorizationId,
        adapter: AdapterInstanceId,
        identity: WorkspaceObjectIdentity,
    }
    impl AuthorizedWorkspaceHandle for Root {
        fn workspace_id(&self) -> &WorkspaceId {
            &self.workspace
        }
        fn authorization_id(&self) -> &WorkspaceAuthorizationId {
            &self.authorization
        }
        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            &self.adapter
        }
        fn platform(&self) -> PathPlatform {
            PathPlatform::DeterministicFake
        }
    }
    impl HeldWorkspaceRoot for Root {
        fn root_identity(&self) -> &WorkspaceObjectIdentity {
            &self.identity
        }
    }
    fn root() -> Root {
        Root {
            workspace: WorkspaceId::from_raw("workspace-0001"),
            authorization: WorkspaceAuthorizationId::from_raw("authorization-0001"),
            adapter: AdapterInstanceId::from_raw("adapter-0001"),
            identity: WorkspaceObjectIdentity::new(
                PathPlatform::DeterministicFake,
                [1; 32],
                [2; 32],
            ),
        }
    }
    fn packet(task: &str) -> PreparedPublicGet {
        let scope = ResearchScope::new(
            task.into(),
            ResearchDepth::Quick,
            ResearchNetworkMode::Ask,
            ResearchLimits::ceiling(ResearchDepth::Quick),
            BTreeSet::from(["docs.example.com".into()]),
            &["public query".into()],
        )
        .unwrap();
        PreparedPublicGet::prepare(
            &scope,
            PublicGetDraft {
                schema_version: 1,
                operation_id: "call-0001".into(),
                target: PublicGetTarget {
                    domain: "docs.example.com".into(),
                    path: "/api".into(),
                    query: vec![("q".into(), "public query".into())],
                },
                maximum_response_bytes: 1024,
                redirect_limit: 0,
                timeout_ms: 1000,
            },
            1000,
            3000,
        )
        .unwrap()
    }
    fn definition() -> ToolDefinition {
        let operation = OperationBinding::new(GrantOperation::NetworkAccess);
        let schema = SchemaReference {
            schema_id: SchemaId::from_raw("public-get-fixture"),
            schema_version: 1,
            schema_sha256: "1".repeat(64),
        };
        ToolDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_id: ToolId::from_raw("research.fixture-get"),
            tool_version: "1.0.0".into(),
            display_name: "Synthetic GET".into(),
            description: "No network execution".into(),
            input_schema: schema.clone(),
            output_schema: schema,
            risk_level: ToolRiskLevel::Moderate,
            declared_effects: vec![operation],
            required_grant: RequiredGrantTemplate {
                operation,
                target_scope: "exact-held-workspace".into(),
                single_use: true,
            },
            timeout_ms: 1000,
        }
    }
    fn call() -> ToolCall {
        let definition = definition();
        let bytes = serde_json::to_vec(packet("task-0001").packet().request()).unwrap();
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-0001"),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            action_id: ActionId::from_raw("action-0001"),
            tool_id: definition.tool_id,
            tool_version: definition.tool_version,
            arguments: ContractPayload {
                schema: definition.input_schema,
                media_type: "application/json".into(),
                sha256: digest(&bytes),
                bytes,
            },
        }
    }
    fn binding() -> PublicGetEffectBinding {
        PublicGetEffectBinding::prepare(&definition(), call(), &root(), packet("task-0001"))
            .unwrap()
    }

    #[test]
    fn exact_call_preparation_is_inert_and_content_minimized() {
        let binding = binding();
        assert_eq!(binding.packet().request().operation_id, "call-0001");
        let debug = format!("{binding:?}");
        for canary in [
            "public query",
            "docs.example.com",
            "task-0001",
            "workspace-0001",
        ] {
            assert!(!debug.contains(canary));
        }
    }

    #[test]
    fn closed_request_and_registered_definition_mismatches_refuse_preparation() {
        for kind in 0..10 {
            let mut call = call();
            let mut definition = definition();
            match kind {
                0 => call.tool_call_id = ToolCallId::from_raw("another-call"),
                1 => call.arguments.media_type = "text/plain".into(),
                2 => call.arguments.sha256 = "0".repeat(64),
                3 => call.tool_version = "2.0.0".into(),
                4 => call.arguments.schema.schema_sha256 = "2".repeat(64),
                5 => definition.required_grant.single_use = false,
                6 => definition.timeout_ms = 999,
                7 => {
                    let mut value: serde_json::Value =
                        serde_json::from_slice(&call.arguments.bytes).unwrap();
                    value["unapproved"] = serde_json::json!(true);
                    call.arguments.bytes = serde_json::to_vec(&value).unwrap();
                    call.arguments.sha256 = digest(&call.arguments.bytes);
                }
                8 => {
                    call.arguments.bytes = call
                        .arguments
                        .bytes
                        .iter()
                        .copied()
                        .chain(b"{}".iter().copied())
                        .collect();
                    call.arguments.sha256 = digest(&call.arguments.bytes);
                }
                9 => {
                    definition
                        .declared_effects
                        .push(OperationBinding::new(GrantOperation::WorkspaceRead));
                }
                _ => unreachable!(),
            }
            assert_eq!(
                PublicGetEffectBinding::prepare(&definition, call, &root(), packet("task-0001"))
                    .err(),
                Some(ResearchEffectBindingError::Call),
                "mutation {kind}"
            );
        }
    }

    #[test]
    fn original_native_json_is_bound_without_forcing_internal_struct_field_order() {
        let mut call = call();
        let value: serde_json::Value = serde_json::from_slice(&call.arguments.bytes).unwrap();
        let original = serde_json::to_vec_pretty(&value).unwrap();
        assert_ne!(original, call.arguments.bytes);
        call.arguments.sha256 = digest(&original);
        call.arguments.bytes = original.clone();
        let prepared =
            PublicGetEffectBinding::prepare(&definition(), call, &root(), packet("task-0001"))
                .unwrap();
        assert_eq!(prepared.call.arguments.bytes, original);
    }

    struct FixtureTool(ToolDefinition);
    impl Tool for FixtureTool {
        fn definition(&self) -> &ToolDefinition {
            &self.0
        }
    }
    fn rules<T: Ord>(value: T) -> ScopeRules<T> {
        ScopeRules {
            allowed: BTreeSet::from([value]),
            denied: BTreeSet::new(),
        }
    }
    struct Driver {
        binding: PublicGetEffectBinding,
        root: Root,
        now: u64,
        checked: usize,
        accepted: usize,
        error: Option<ResearchEffectBindingError>,
    }
    impl EffectDriver for Driver {
        fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
            self.checked += 1;
            if let Err(error) = self.binding.validate(&authorization, &self.root, self.now) {
                self.error = Some(error);
                return EffectLaunch::failed();
            }
            self.accepted += 1;
            // Synthetic observation only. No worker, DNS, socket or HTTP request.
            EffectLaunch::completed(EffectResult::from_redacted_material(
                OperationOutcome::Succeeded,
                b"synthetic-permit-match",
                StateChange::NotChanged,
            ))
        }
    }

    fn exercise(
        driver: Driver,
        operation: GrantOperation,
        network: &str,
        details: &str,
        expired: bool,
    ) -> Driver {
        exercise_with_cancellation(driver, operation, network, details, expired, false)
    }

    fn exercise_with_cancellation(
        driver: Driver,
        operation: GrantOperation,
        network: &str,
        details: &str,
        expired: bool,
        cancelled: bool,
    ) -> Driver {
        exercise_with_exact_authority(
            driver,
            operation,
            network,
            details,
            cancelled,
            GrantTimingFixture {
                call: call(),
                consumed_at: if expired { 31000 } else { 3000 },
                expires_at: 30000,
            },
        )
    }

    struct GrantTimingFixture {
        call: ToolCall,
        consumed_at: u64,
        expires_at: u64,
    }

    fn exercise_with_exact_authority(
        mut driver: Driver,
        operation: GrantOperation,
        network: &str,
        details: &str,
        cancelled: bool,
        timing: GrantTimingFixture,
    ) -> Driver {
        let mut definition = definition();
        let operation = OperationBinding::new(operation);
        definition.declared_effects = vec![operation];
        definition.required_grant.operation = operation;
        let call = timing.call;
        let root = root();
        let target = GrantTarget::held_workspace_root(&root).unwrap();
        let actor = ActorId::from_raw("actor-0001");
        let task = TaskId::from_raw("task-0001");
        let session = SessionId::from_raw("session-0001");
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules(actor.clone()),
            tasks: rules(task.clone()),
            actions: rules(call.action_id.clone()),
            tools: rules(ToolPolicyBinding {
                tool_id: call.tool_id.clone(),
                tool_version: call.tool_version.clone(),
            }),
            operations: rules(operation),
            targets: rules(target.clone()),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: rules(network.into()),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
        .unwrap();
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FixtureTool(definition)))
            .unwrap();
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-0001"),
                actor_id: actor.clone(),
                session_id: session.clone(),
                task_id: task.clone(),
                targets: vec![scope(&[])],
                excluded_targets: vec![],
                sensitivity: DataSensitivity::Ephemeral,
                issued_at_epoch_ms: 1000,
                expires_at_epoch_ms: 60000,
                nonce: GrantNonce::from_raw("nonce-parent-0001"),
                maximum_derived_operations: 1,
                preview_sha256: "2".repeat(64),
                policy_sha256: policy.policy_sha256().into(),
            })
            .unwrap();
        let approval = ApprovalId::from_raw("approval-0001");
        let grant = issuer
            .derive_operation(
                &parent.grant_id,
                DerivedOperationGrantRequest {
                    grant_id: GrantId::from_raw("grant-operation-0001"),
                    approval_id: approval.clone(),
                    action_id: call.action_id.clone(),
                    action_kind: ActionKind::DeterministicTool,
                    operation,
                    tool_id: call.tool_id.clone(),
                    tool_version: call.tool_version.clone(),
                    targets: vec![target],
                    argument_sha256: call.arguments.sha256.clone(),
                    preimages: vec![],
                    expected_side_effects: vec![GrantSideEffect {
                        operation,
                        target_indexes: vec![0],
                        details_sha256: details.into(),
                    }],
                    rollback_description: "External disclosure cannot be undone".into(),
                    issued_at_epoch_ms: 2000,
                    expires_at_epoch_ms: timing.expires_at,
                    nonce: GrantNonce::from_raw("nonce-operation-0001"),
                    preview_sha256: "3".repeat(64),
                    policy_sha256: policy.policy_sha256().into(),
                },
            )
            .unwrap();
        if cancelled {
            let issued_sha256 = issuer
                .revision_hash(&grant.grant_id, grant.revision)
                .unwrap()
                .to_owned();
            issuer
                .cancel_before_execution(&grant.grant_id, &issued_sha256, 2500)
                .unwrap();
        }
        let context = PolicyEvaluationContext {
            actor_id: actor,
            session_id: session,
            task_id: task,
            action_id: call.action_id.clone(),
            action_kind: ActionKind::DeterministicTool,
            tool_id: call.tool_id.clone(),
            tool_version: call.tool_version.clone(),
            targets: grant.targets.clone(),
            argument_sha256: grant.argument_sha256.clone(),
            preimages: grant.preimages.clone(),
            expected_side_effects: grant.expected_side_effects.clone(),
            preview_sha256: grant.preview_sha256.clone(),
            now_epoch_ms: timing.consumed_at,
            // Let a valid non-network grant reach the binding's operation check.
            // Attaching a network scope to it would instead be denied earlier by
            // policy, leaving that independent driver restriction unexercised.
            network_scope: (operation.operation() == GrantOperation::NetworkAccess)
                .then(|| network.into()),
            credential_scope: None,
            publication_scope: None,
        };
        let replay = AuthorityTransactionRequest::new(
            AuthorityTransactionId::from_raw("transaction-replay"),
            OperationAttemptId::from_raw("attempt-replay"),
            approval.clone(),
            grant.grant_id.clone(),
            call.clone(),
            context.clone(),
            32000,
            "1970-01-01T00:00:32Z",
        )
        .unwrap();
        let request = AuthorityTransactionRequest::new(
            AuthorityTransactionId::from_raw("transaction-0001"),
            OperationAttemptId::from_raw("attempt-0001"),
            approval,
            grant.grant_id,
            call,
            context,
            32000,
            "1970-01-01T00:00:32Z",
        )
        .unwrap();
        let mut coordinator = AuthorityTransactionCoordinator::new();
        coordinator
            .execute_effect(&registry, &mut issuer, &policy, request, &mut driver)
            .unwrap();
        let checks = driver.checked;
        // A different transaction/attempt cannot reuse this same consumed or
        // invalidated grant, even when all original call and packet bytes match.
        coordinator
            .execute_effect(&registry, &mut issuer, &policy, replay, &mut driver)
            .unwrap();
        assert_eq!(
            driver.checked, checks,
            "grant replay must not reach the driver"
        );
        driver
    }
    fn driver() -> Driver {
        Driver {
            binding: binding(),
            root: root(),
            now: 3000,
            checked: 0,
            accepted: 0,
            error: None,
        }
    }

    #[test]
    fn actual_consumed_permit_is_required_and_all_exact_bindings_match() {
        let driver = driver();
        let details = driver.binding.packet().sha256().to_owned();
        let driver = exercise(
            driver,
            GrantOperation::NetworkAccess,
            "https:docs.example.com:443",
            &details,
            false,
        );
        assert_eq!((driver.checked, driver.accepted), (1, 1));
        assert_eq!(driver.error, None);
    }

    #[test]
    fn disclosure_scope_operation_task_and_held_root_drift_refuse_after_consumption() {
        for kind in 0..7 {
            let mut driver = driver();
            let mut details = driver.binding.packet().sha256().to_owned();
            let mut operation = GrantOperation::NetworkAccess;
            let mut network = "https:docs.example.com:443";
            match kind {
                0 => details = "4".repeat(64),
                1 => network = "https:other.example.com:443",
                2 => operation = GrantOperation::WorkspaceRead,
                3 => {
                    driver.binding.prepared = packet("task-other");
                    // Keep the side-effect digest correct for this packet so
                    // only the independently consumed task identity differs.
                    details = driver.binding.packet().sha256().to_owned();
                }
                4 => {
                    driver.root.identity = WorkspaceObjectIdentity::new(
                        PathPlatform::DeterministicFake,
                        [1; 32],
                        [9; 32],
                    )
                }
                5 => {
                    driver.binding.call.correlation_id =
                        CorrelationId::from_raw("correlation-other")
                }
                6 => driver.binding.call.arguments.schema.schema_sha256 = "5".repeat(64),
                _ => unreachable!(),
            }
            let driver = exercise(driver, operation, network, &details, false);
            assert_eq!((driver.checked, driver.accepted), (1, 0), "mutation {kind}");
            assert_eq!(
                driver.error,
                Some(ResearchEffectBindingError::Authority),
                "mutation {kind}"
            );
        }
    }

    #[test]
    fn expired_grant_never_reaches_driver_and_packet_clock_window_cannot_extend() {
        let original = driver();
        let details = original.binding.packet().sha256().to_owned();
        let expired = exercise(
            original,
            GrantOperation::NetworkAccess,
            "https:docs.example.com:443",
            &details,
            true,
        );
        assert_eq!((expired.checked, expired.accepted), (0, 0));
        for now in [2999, 4000] {
            let mut driver = driver();
            driver.now = now;
            let driver = exercise(
                driver,
                GrantOperation::NetworkAccess,
                "https:docs.example.com:443",
                &details,
                false,
            );
            assert_eq!((driver.checked, driver.accepted), (1, 0));
            assert_eq!(driver.error, Some(ResearchEffectBindingError::Deadline));
        }
    }

    #[test]
    fn cancelled_authority_cannot_be_revived_by_an_exact_request_or_fresh_attempt() {
        let driver = driver();
        let details = driver.binding.packet().sha256().to_owned();
        let driver = exercise_with_cancellation(
            driver,
            GrantOperation::NetworkAccess,
            "https:docs.example.com:443",
            &details,
            false,
            true,
        );
        assert_eq!((driver.checked, driver.accepted), (0, 0));
    }

    #[test]
    fn valid_launch_clock_cannot_extend_the_exact_consumed_grant_deadline() {
        for now in [3000, 3499, 3500, 3999] {
            let mut driver = driver();
            driver.now = now;
            assert_eq!(driver.binding.packet().deadline_epoch_ms(), 4000);
            let details = driver.binding.packet().sha256().to_owned();
            let driver = exercise_with_exact_authority(
                driver,
                GrantOperation::NetworkAccess,
                "https:docs.example.com:443",
                &details,
                false,
                GrantTimingFixture {
                    call: call(),
                    consumed_at: 3000,
                    expires_at: 3500,
                },
            );
            assert_eq!((driver.checked, driver.accepted), (1, 0), "launch {now}");
            assert_eq!(driver.error, Some(ResearchEffectBindingError::Deadline));
        }
    }

    #[test]
    fn equal_packet_and_grant_expiry_is_exclusive_and_consumption_clock_is_monotonic() {
        for (consumed_at, now, accepted) in [
            (3000, 3000, true),
            (3000, 3999, true),
            (3000, 4000, false),
            (3100, 3099, false),
            (3100, 3100, true),
        ] {
            let mut driver = driver();
            driver.now = now;
            let details = driver.binding.packet().sha256().to_owned();
            let driver = exercise_with_exact_authority(
                driver,
                GrantOperation::NetworkAccess,
                "https:docs.example.com:443",
                &details,
                false,
                GrantTimingFixture {
                    call: call(),
                    consumed_at,
                    expires_at: 4000,
                },
            );
            assert_eq!(
                (driver.checked, driver.accepted),
                (1, usize::from(accepted))
            );
            assert_eq!(
                driver.error,
                (!accepted).then_some(ResearchEffectBindingError::Deadline)
            );
        }
        let driver = driver();
        let details = driver.binding.packet().sha256().to_owned();
        let driver = exercise_with_exact_authority(
            driver,
            GrantOperation::NetworkAccess,
            "https:docs.example.com:443",
            &details,
            false,
            GrantTimingFixture {
                call: call(),
                consumed_at: 4000,
                expires_at: 4000,
            },
        );
        assert_eq!((driver.checked, driver.accepted), (0, 0));
    }

    #[test]
    fn shorter_packet_is_prepared_before_approval_not_rewritten_after_consumption() {
        for now in [3000, 3499, 3500] {
            let original = packet("task-0001");
            let mut request = original.packet().request().clone();
            request.timeout_ms = 500;
            let scope = ResearchScope::new(
                "task-0001".into(),
                ResearchDepth::Quick,
                ResearchNetworkMode::Ask,
                ResearchLimits::ceiling(ResearchDepth::Quick),
                BTreeSet::from(["docs.example.com".into()]),
                &["public query".into()],
            )
            .unwrap();
            let prepared = PreparedPublicGet::prepare(&scope, request, 1000, 3000).unwrap();
            assert_eq!(prepared.packet().deadline_epoch_ms(), 3500);
            let mut approved_call = call();
            approved_call.arguments.bytes =
                serde_json::to_vec(prepared.packet().request()).unwrap();
            approved_call.arguments.sha256 = digest(&approved_call.arguments.bytes);
            let binding = PublicGetEffectBinding::prepare(
                &definition(),
                approved_call.clone(),
                &root(),
                prepared,
            )
            .unwrap();
            let original_packet = binding.packet().sha256().to_owned();
            let driver = Driver {
                binding,
                root: root(),
                now,
                checked: 0,
                accepted: 0,
                error: None,
            };
            let driver = exercise_with_exact_authority(
                driver,
                GrantOperation::NetworkAccess,
                "https:docs.example.com:443",
                &original_packet,
                false,
                GrantTimingFixture {
                    call: approved_call.clone(),
                    consumed_at: 3000,
                    expires_at: 3500,
                },
            );
            assert_eq!(
                (driver.checked, driver.accepted),
                (1, usize::from(now < 3500))
            );
            assert_eq!(
                driver.error,
                (now >= 3500).then_some(ResearchEffectBindingError::Deadline)
            );
            assert_eq!(driver.binding.call, approved_call);
            assert_eq!(driver.binding.packet().sha256(), original_packet);
        }
    }
}
