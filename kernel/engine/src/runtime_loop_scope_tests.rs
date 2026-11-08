// SPDX-License-Identifier: Apache-2.0
//! Synthetic integration with the existing coordinator, never a coding score.

use super::*;
use crate::runtime_loop::{RuntimeRunScope, RuntimeScopedVerificationInput};
use agentmage_kernel_contracts::{
    ClosedModelProposal, EndpointModelSelection, ModelRunId, RuntimeEndpointContextBudget,
    RuntimeEndpointRunRequest, RuntimeModelBinding,
};

fn endpoint(native: &RuntimeRunRequest) -> RuntimeEndpointRunRequest {
    crate::runtime_coordinator::seal_runtime_endpoint_run_request(RuntimeEndpointRunRequest {
        schema_version: native.schema_version,
        run_id: native.run_id.clone(),
        session_id: native.session_id.clone(),
        mode: native.mode,
        task: native.task.clone(),
        work_packet: native.work_packet.clone(),
        workspace_id: native.workspace_id.clone(),
        workspace_snapshot_sha256: native.workspace_snapshot_sha256.clone(),
        repository_snapshot_id: native.repository_snapshot_id.clone(),
        repository_snapshot_sha256: native.repository_snapshot_sha256.clone(),
        model_selection: EndpointModelSelection {
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: "synthetic".into(),
        },
        context_budget: RuntimeEndpointContextBudget {
            max_messages: 2,
            max_content_bytes: 128,
            max_serialized_messages_bytes: 256,
            max_request_body_bytes: 512,
        },
        max_output_tokens: 16,
        tool_catalog_id: native.tool_catalog_id.clone(),
        tool_catalog_sha256: native.tool_catalog_sha256.clone(),
        visible_tools: native.visible_tools.clone(),
        policy_id: native.policy_id.clone(),
        policy_sha256: native.policy_sha256.clone(),
        limits: native.limits.clone(),
        event_cursor: None,
        request_sha256: "0".repeat(64),
    })
    .unwrap()
}

#[test]
fn run_scope_preserves_complete_request_family_and_common_observations() {
    let (coordinator, _) = coordinator([], PermissionScript::Allow, true);
    let native = &coordinator.request;
    let endpoint = endpoint(native);
    for scope in [
        RuntimeRunScope::native(native).unwrap(),
        RuntimeRunScope::endpoint(&endpoint).unwrap(),
    ] {
        assert_eq!(scope.run_id(), &native.run_id);
        assert_eq!(scope.session_id(), &native.session_id);
        assert_eq!(scope.mode(), native.mode);
        assert_eq!(scope.task(), &native.task);
        assert_eq!(scope.work_packet(), &native.work_packet);
        assert_eq!(scope.workspace_id(), &native.workspace_id);
        assert_eq!(
            scope.workspace_snapshot_sha256(),
            native.workspace_snapshot_sha256
        );
        assert_eq!(
            scope.repository_snapshot_id(),
            &native.repository_snapshot_id
        );
        assert_eq!(
            scope.repository_snapshot_sha256(),
            native.repository_snapshot_sha256
        );
        assert_eq!(scope.tool_catalog_id(), &native.tool_catalog_id);
        assert_eq!(scope.tool_catalog_sha256(), native.tool_catalog_sha256);
        assert_eq!(scope.visible_tools(), native.visible_tools);
        assert_eq!(scope.policy_id(), &native.policy_id);
        assert_eq!(scope.policy_sha256(), native.policy_sha256);
        assert_eq!(scope.limits(), &native.limits);
    }
    let scope = RuntimeRunScope::native(native).unwrap();
    assert!(std::ptr::eq(scope.native_request().unwrap(), native));
    assert!(scope.endpoint_request().is_none());
    assert_eq!(
        scope.model_binding(),
        RuntimeModelBinding::ExactProfile(&native.model_profile)
    );
    assert_eq!(scope.request_sha256(), native.request_sha256);
    let scope = RuntimeRunScope::endpoint(&endpoint).unwrap();
    assert_eq!(scope.native_request(), Err(RuntimePortFailure::Invalid));
    assert!(std::ptr::eq(scope.endpoint_request().unwrap(), &endpoint));
    assert_eq!(
        scope.model_binding(),
        RuntimeModelBinding::SelectedEndpoint {
            endpoint: &endpoint.model_selection.endpoint,
            model: &endpoint.model_selection.model,
        }
    );
    assert_eq!(scope.request_sha256(), endpoint.request_sha256);
    assert_ne!(scope.request_sha256(), native.request_sha256);
}

#[test]
fn run_scope_refuses_drift_and_resealed_invalid_native_admission() {
    let (coordinator, _) = coordinator([], PermissionScript::Allow, true);
    let native = coordinator.request;
    let original_endpoint = endpoint(&native);
    let mut bad = original_endpoint.clone();
    bad.policy_sha256 = "d".repeat(64);
    assert!(RuntimeRunScope::endpoint(&bad).is_err());
    let mut bad = original_endpoint.clone();
    bad.model_selection.model = "other".into();
    assert!(RuntimeRunScope::endpoint(&bad).is_err());
    let mut bad = native.clone();
    bad.model_profile.context.token_counter_sha256 = "d".repeat(64);
    assert!(RuntimeRunScope::native(&bad).is_err());
    // Even a correctly recomputed digest cannot make an invalid native tuple
    // admissible. This retains native shape validation, not just hash checking.
    bad.request_sha256 = "0".repeat(64);
    bad.request_sha256 = sha256(&to_canonical_json(&bad).unwrap());
    assert!(RuntimeRunScope::native(&bad).is_err());
}

struct NativeOnly;
impl RuntimeToolBoundary for NativeOnly {
    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("native-only evaluate must not see an endpoint")
    }
    fn resolve(
        &mut self,
        _: &RuntimeRunRequest,
        _: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        _: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("native-only resolve must not see an endpoint")
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        panic!("native-only execute must not see an endpoint")
    }
}

impl RuntimeVerifierPort for NativeOnly {
    fn verifier_id(&self) -> &VerifierId {
        panic!("not requested")
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("native-only verifier must not see an endpoint")
    }
}

fn completion(request: &RuntimeRunRequest) -> ClosedModelProposal {
    let mut model = FakeModel::new(request.model_profile.clone(), [ModelScript::Completion]);
    let context = FakeContext
        .build_context(
            request,
            ContextPacketId::from_raw("context-0001"),
            1,
            &[],
            &[],
            &[],
        )
        .unwrap();
    model
        .run_model(
            &ModelRunRequest {
                schema_version: CONTRACT_SCHEMA_VERSION,
                model_run_id: ModelRunId::from_raw("model-run-0001"),
                correlation_id: CorrelationId::from_raw("correlation-0001"),
                context_packet_id: context.context_packet_id.clone(),
                profile_id: request.model_profile.profile_id.clone(),
                manifest_sha256: request.model_profile.manifest_sha256.clone(),
                adapter_id: request.model_profile.runtime.adapter_id.clone(),
                decoding_profile_id: request.model_profile.decoding.profile_id.clone(),
                max_output_tokens: 16,
                timeout_ms: 1000,
            },
            &context,
            None,
        )
        .unwrap()
        .proposal
        .unwrap()
}

#[test]
fn run_scope_legacy_tool_and_verifier_ports_refuse_endpoint_before_entry() {
    let (mut coordinator, _) = coordinator([ModelScript::Tool], PermissionScript::Ask, true);
    let RuntimeCoordinatorStep::AwaitingApproval { challenge } =
        coordinator.run_until_boundary(None, None).unwrap()
    else {
        panic!("approval")
    };
    let endpoint = endpoint(&coordinator.request);
    let scope = RuntimeRunScope::endpoint(&endpoint).unwrap();
    let pending = coordinator.pending.as_ref().unwrap();
    let response = decide(&challenge, RuntimeApprovalDisposition::Allow);
    let mut legacy = NativeOnly;
    assert_eq!(
        legacy.evaluate_scoped(
            scope,
            &pending.operation_id,
            &pending.definition,
            &pending.call,
            1000
        ),
        Err(RuntimePortFailure::Invalid)
    );
    assert_eq!(
        legacy.resolve_scoped(
            scope,
            &challenge,
            &response,
            &pending.definition,
            &pending.call,
            1000
        ),
        Err(RuntimePortFailure::Invalid)
    );
    let evaluation = RuntimePermissionEvaluation::Ask {
        approval_id: challenge.approval_id.clone(),
        grant_id: challenge.proposed_grant_id.clone(),
        preview_sha256: challenge.preview_sha256.clone(),
        expires_at_epoch_ms: challenge.expires_at_epoch_ms,
    };
    assert_eq!(
        legacy.execute_scoped(scope, &evaluation, &pending.definition, &pending.call, None),
        Err(RuntimePortFailure::Invalid)
    );
    let proposal = completion(&coordinator.request);
    assert_eq!(
        legacy.verify_scoped(RuntimeVerificationInput {
            request: scope,
            proposal: &proposal,
            state_revision: 8,
            postconditions: &[],
            evidence: &[],
            tool_results: &[],
            receipt_ids: &[]
        }),
        Err(RuntimePortFailure::Invalid)
    );
}

struct ScopedBoundary {
    inner: FakeToolBoundary,
    calls: Arc<Mutex<Vec<&'static str>>>,
}
impl RuntimeToolBoundary for ScopedBoundary {
    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("coordinator must use scoped evaluate")
    }
    fn resolve(
        &mut self,
        _: &RuntimeRunRequest,
        _: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        _: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("coordinator must use scoped resolve")
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        panic!("coordinator must use scoped execute")
    }
    fn evaluate_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        operation: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.calls.lock().unwrap().push("evaluate");
        self.inner
            .evaluate(scope.native_request()?, operation, definition, call, now)
    }
    fn resolve_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        challenge: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.calls.lock().unwrap().push("resolve");
        self.inner.resolve(
            scope.native_request()?,
            challenge,
            response,
            definition,
            call,
            now,
        )
    }
    fn execute_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        self.calls.lock().unwrap().push("execute");
        self.inner.execute(
            scope.native_request()?,
            evaluation,
            definition,
            call,
            cancellation,
        )
    }
}

struct ScopedVerifier {
    inner: FakeVerifier,
    calls: Arc<Mutex<Vec<&'static str>>>,
}
impl RuntimeVerifierPort for ScopedVerifier {
    fn verifier_id(&self) -> &VerifierId {
        self.inner.verifier_id()
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("coordinator must use scoped verify")
    }
    fn verify_scoped(
        &mut self,
        input: RuntimeScopedVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        self.calls.lock().unwrap().push("verify");
        let request = input.request.native_request()?;
        // The default adapter is tested by the old coordinator cases. This
        // override proves the actual coordinator selects its new entry point.
        self.inner.verify(input.with_request(request))
    }
}

#[test]
fn run_scope_existing_coordinator_uses_scoped_approval_execution_and_verification() {
    let (original, executions) = coordinator(
        [ModelScript::Tool, ModelScript::Completion],
        PermissionScript::Ask,
        true,
    );
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = ReusableRuntimeCoordinator::new(
        original.request,
        original.model,
        original.context,
        original.registry,
        ScopedBoundary {
            inner: original.tool_boundary,
            calls: calls.clone(),
        },
        ScopedVerifier {
            inner: original.verifier,
            calls: calls.clone(),
        },
        original.clock,
    )
    .unwrap();
    let RuntimeCoordinatorStep::AwaitingApproval { challenge } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("approval")
    };
    assert_eq!(*calls.lock().unwrap(), ["evaluate"]);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    let mut response = decide(&challenge, RuntimeApprovalDisposition::Allow);
    response.challenge_sha256 = "d".repeat(64);
    assert!(runtime.run_until_boundary(Some(&response), None).is_err());
    assert_eq!(*calls.lock().unwrap(), ["evaluate"]);
    let response = decide(&challenge, RuntimeApprovalDisposition::Allow);
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(Some(&response), None).unwrap()
    else {
        panic!("complete")
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert_eq!(
        *calls.lock().unwrap(),
        ["evaluate", "resolve", "execute", "verify"]
    );
    let mut sequence = RuntimeEventSequence::new();
    for event in runtime.events() {
        sequence.push(event).unwrap();
    }
}
