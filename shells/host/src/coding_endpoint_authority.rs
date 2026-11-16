// SPDX-License-Identifier: MIT OR Apache-2.0
//! Inert policy binding for endpoint runs over the original kernel authority machinery.

use agentmage_kernel_contracts::{RuntimeEndpointRunRequest, RuntimeSessionMode};
use agentmage_kernel_engine::{
    runtime_coordinator::{
        runtime_tool_catalog_sha256, seal_runtime_endpoint_run_request,
        verify_runtime_endpoint_run_request,
    },
    runtime_loop::runtime_tool_references,
};

use super::*;

/// Trusted startup inputs, never a model proposal or wire authority object.
/// Source consent, held workspace admission and registry selection belong to
/// the caller. No native model profile is constructed or admitted here.
pub struct EndpointCodingPolicyRequest<'a, Workspace: AuthorizedWorkspaceHandle> {
    /// Original sealed, not-yet-admitted endpoint run. Only its policy binding
    /// and consequent whole-request seal are replaced in the returned value.
    pub request: RuntimeEndpointRunRequest,
    /// Exact person-facing actor for later protected decisions.
    pub actor_id: &'a ActorId,
    /// Continuously held platform authorization, not an absolute path string.
    pub workspace: &'a Workspace,
    /// Complete exact catalog already named by the sealed request.
    pub registry: &'a ToolRegistry,
    /// Trusted explicit exclusions. These are canonicalized without dropping any.
    pub excluded_scopes: Vec<WorkspaceScopePath>,
}

/// One resealed endpoint request and its matching deny-by-default policy.
/// This is descriptive startup composition: no grant has been issued, approved
/// or consumed, no path read and no coordinator or model started. Keep both
/// values together when composing the eventual tool boundary and coordinator.
pub struct BoundEndpointCodingPolicy {
    /// Complete original request with exactly the returned policy binding.
    pub request: RuntimeEndpointRunRequest,
    /// Same policy type used by native coding approvals and grant consumption.
    pub policy: CodingRuntimePolicy,
}

/// Binds an original endpoint request to the same native kernel policy builder,
/// using held workspace identity, exact tool definitions and bounded actions.
/// Only ephemeral, non-resumed read-only runs are supported. This does not
/// enable the native-profile Linux boundary or establish filesystem execution,
/// durable authority, model qualification or completion verification.
pub fn bind_endpoint_coding_policy<Workspace: AuthorizedWorkspaceHandle>(
    input: EndpointCodingPolicyRequest<'_, Workspace>,
) -> Result<BoundEndpointCodingPolicy, CodingApprovalError> {
    let mut request = input.request;
    verify_runtime_endpoint_run_request(&request).map_err(|_| CodingApprovalError::PolicyDenied)?;
    let tools =
        runtime_tool_references(input.registry).map_err(|_| CodingApprovalError::PolicyDenied)?;
    if request.mode != RuntimeSessionMode::EphemeralReadOnly
        || request.event_cursor.is_some()
        || request.workspace_id != *input.workspace.workspace_id()
        || tools.is_empty()
        || tools != request.visible_tools
        || runtime_tool_catalog_sha256(&request.tool_catalog_id, &tools)
            .map_err(|_| CodingApprovalError::PolicyDenied)?
            != request.tool_catalog_sha256
        || input.registry.list_tools().iter().any(|definition| {
            definition.required_grant.operation.operation() != GrantOperation::WorkspaceRead
                || definition
                    .declared_effects
                    .iter()
                    .any(|effect| effect.operation() != GrantOperation::WorkspaceRead)
        })
    {
        return Err(CodingApprovalError::PolicyDenied);
    }
    let policy = build_runtime_policy_envelope(RuntimePolicyEnvelopeRequest {
        actor_id: input.actor_id,
        task_id: &request.task.task_id,
        run_id: &request.run_id,
        workspace: input.workspace,
        registry: input.registry,
        maximum_tool_calls: request.limits.max_tool_calls,
        excluded_scopes: input.excluded_scopes,
    })?;
    request.policy_id = policy.policy_id().clone();
    request.policy_sha256 = policy.engine().policy_sha256().to_owned();
    let request = seal_runtime_endpoint_run_request(request)
        .map_err(|_| CodingApprovalError::PolicyDenied)?;
    Ok(BoundEndpointCodingPolicy { request, policy })
}

#[cfg(all(test, target_os = "linux"))]
#[path = "coding_endpoint_authority_tests.rs"]
mod tests;
