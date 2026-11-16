// SPDX-License-Identifier: MIT OR Apache-2.0
//! Trusted startup composition, without a native model profile or a second loop.

use std::sync::Arc;

use agentmage_capability_repository_map::coding_proposal::CodingSource;
use agentmage_kernel_contracts::{
    ActorId, AuthorizedWorkspaceHandle, RuntimeEndpointRunRequest, WorkspacePath,
};
use agentmage_kernel_engine::{
    instruction_provenance::{InstructionEvidenceLedger, effective_guidance},
    runtime_coordinator::{
        runtime_tool_catalog_sha256, seal_runtime_endpoint_run_request,
        verify_runtime_endpoint_run_request,
    },
    runtime_loop::{RuntimeClock, RuntimeVerifierPort, runtime_tool_references},
    tooling::ToolRegistry,
};
use agentmage_platform_linux::{
    LinuxAuthorityRuntime, LinuxAuthorizedWorkspace, LinuxSandboxRunner,
};

use crate::{
    coding_authority::{
        BoundEndpointCodingPolicy, EndpointCodingPolicyRequest, bind_endpoint_coding_policy,
    },
    coding_endpoint::CodingEndpointConnection,
    coding_endpoint_files::{CodingEndpointFiles, CodingEndpointFilesInput},
    coding_endpoint_owner::{
        CodingEndpointAdmission, CodingEndpointOwnerError, CodingEndpointRuns,
    },
    coding_endpoint_read_verifier::{CodingEndpointReadCheck, CodingEndpointReadVerifier},
    coding_guidance::{CodingGuidanceError, CodingGuidancePolicy, compile_coding_read_guidance},
    linux_coding_runtime::CodingIdentitySource,
    runtime_tools::workspace_file_read_runtime_registry,
};

/// Content-free startup refusals. Construction and admission execute no model or tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingEndpointStartupError {
    /// Request, workspace, catalog or policy does not match the original selection.
    Binding,
    /// Repository provenance is invalid, stale, conflicting or for another snapshot.
    Instructions,
    /// An original trusted narrowing constraint cannot be honored by this boundary.
    Guidance(CodingGuidanceError),
    /// Narrowing removed the only supported file tool, or its usable call budget.
    NoTools,
    /// A supplied source path is malformed or falls within an excluded scope.
    Source,
    /// Exact file-observation checks are excluded or do not cover this request.
    Acceptance,
    /// Original native file/grant dependencies failed their admission.
    Native,
    /// The original retained coordinator owner refused admission.
    Owner(CodingEndpointOwnerError),
}

/// Original native resources and trusted ports. Neither wire arguments nor model
/// output may supply these values. The verifier must independently check this
/// run's acceptance criteria; it is never selected by the model or reconstructed
/// from a candidate. This factory does not qualify an arbitrary verifier.
pub struct CodingEndpointStartInput<'config, 'source, I, V, C> {
    /// Original credential-owning connection and separate destination approval.
    pub connection: CodingEndpointConnection<'config>,
    /// Admitted offline file worker; never a replacement executor.
    pub sandbox: LinuxSandboxRunner,
    /// Original privately rooted durable kernel authority.
    pub authority: LinuxAuthorityRuntime,
    /// Trusted fresh grant/approval/transaction identities.
    pub identities: I,
    /// Trusted deterministic acceptance checks, independent of model narration.
    pub verifier: V,
    /// Original clock, retained across all advances and decisions.
    pub clock: C,
    /// Fresh snapshots captured under prior source-read and disclosure consent.
    /// Excluded paths refuse the whole admission, without silently dropping data.
    pub sources: &'source [CodingSource],
    /// Original deterministic excerpt allocation in bytes, never model tokens.
    pub source_budget: usize,
    /// Request endpoint usage observations, without inventing missing usage.
    pub include_usage: bool,
}

/// Non-cloneable preparation holds the selected Linux workspace continuously.
/// It owns the compiled narrowing, original catalog and matching kernel policy.
/// There is no native model/profile, grant issuance, source discovery or socket
/// activity here. Admission consumes preparation; resume is not supported.
pub struct CodingEndpointStartup {
    bound: BoundEndpointCodingPolicy,
    workspace: Arc<LinuxAuthorizedWorkspace>,
    actor_id: ActorId,
    registry: ToolRegistry,
    guidance: CodingGuidancePolicy,
}

impl CodingEndpointStartup {
    /// Verify the original sealed selection and complete instruction provenance,
    /// compile the existing narrowing machinery, then bind the resulting exact
    /// read-file catalog and ceilings to the original kernel coding policy.
    /// An empty ledger is explicit, not inferred from missing provenance. The
    /// caller must have obtained workspace selection/read consent and captured
    /// the current ledger; a digest alone supplies no such authority or freshness.
    pub fn prepare(
        mut request: RuntimeEndpointRunRequest,
        workspace: Arc<LinuxAuthorizedWorkspace>,
        actor_id: ActorId,
        ledger: &InstructionEvidenceLedger,
    ) -> Result<Self, CodingEndpointStartupError> {
        use CodingEndpointStartupError as Error;
        verify_runtime_endpoint_run_request(&request).map_err(|_| Error::Binding)?;
        let mut registry = workspace_file_read_runtime_registry().map_err(|_| Error::Binding)?;
        if request.workspace_id != *workspace.workspace_id()
            || runtime_tool_references(&registry).map_err(|_| Error::Binding)?
                != request.visible_tools
        {
            return Err(Error::Binding);
        }
        if ledger.workspace_manifest_sha256 != request.workspace_snapshot_sha256 {
            return Err(Error::Instructions);
        }
        let effective = effective_guidance(ledger).map_err(|_| Error::Instructions)?;
        // No command or validation worker is registered by this read-only
        // composition. Required validation constraints therefore refuse, rather
        // than being recorded and forgotten or satisfied by model text.
        let guidance = compile_coding_read_guidance(
            &request.workspace_id,
            ledger,
            &effective,
            &registry,
            &request.limits,
        )
        .map_err(Error::Guidance)?;
        guidance.restrict_registry(&mut registry);
        request.visible_tools = runtime_tool_references(&registry).map_err(|_| Error::Binding)?;
        if request.visible_tools.is_empty() || guidance.limits().max_tool_calls == 0 {
            return Err(Error::NoTools);
        }
        request.limits = guidance.limits().clone();
        request.tool_catalog_sha256 =
            runtime_tool_catalog_sha256(&request.tool_catalog_id, &request.visible_tools)
                .map_err(|_| Error::Binding)?;
        // Retain the complete narrowing identity in the sealed run, including
        // constraints (such as additional confirmation) already enforced by the
        // exact-approval file boundary and therefore not widening its policy.
        request.task.constraints.push(format!(
            "Trusted repository narrowing policy SHA-256: {}",
            guidance.policy_sha256()
        ));
        request = seal_runtime_endpoint_run_request(request).map_err(|_| Error::Binding)?;
        let bound = bind_endpoint_coding_policy(EndpointCodingPolicyRequest {
            request,
            actor_id: &actor_id,
            workspace: &*workspace,
            registry: &registry,
            excluded_scopes: guidance.excluded_scopes().to_vec(),
        })
        .map_err(|_| Error::Binding)?;
        Ok(Self {
            bound,
            workspace,
            actor_id,
            registry,
            guidance,
        })
    }

    /// Exact final request, including compiled narrowing and original kernel policy.
    #[must_use]
    pub const fn request(&self) -> &RuntimeEndpointRunRequest {
        &self.bound.request
    }

    /// Original compiled narrowing; metadata cannot authorize a grant or source read.
    #[must_use]
    pub const fn guidance(&self) -> &CodingGuidancePolicy {
        &self.guidance
    }

    /// Select the concrete full-file observation verifier for this final sealed
    /// request. Every check must already be in the task/work packet and outside
    /// compiled exclusions. This does not verify arbitrary task prose or read a
    /// file; the trusted caller supplies previously consented snapshot identities.
    pub fn read_verifier(
        &self,
        checks: Vec<CodingEndpointReadCheck>,
    ) -> Result<CodingEndpointReadVerifier, CodingEndpointStartupError> {
        if checks.iter().any(|check| {
            self.guidance
                .excluded_scopes()
                .iter()
                .any(|scope| scope.contains_path(check.path()))
        }) {
            return Err(CodingEndpointStartupError::Acceptance);
        }
        CodingEndpointReadVerifier::for_request(self.request(), checks)
            .map_err(|_| CodingEndpointStartupError::Acceptance)
    }

    fn check_sources(&self, sources: &[CodingSource]) -> Result<(), CodingEndpointStartupError> {
        for source in sources {
            let path = WorkspacePath::new(
                self.bound.request.workspace_id.clone(),
                source.path.split('/'),
            )
            .map_err(|_| CodingEndpointStartupError::Source)?;
            if self
                .guidance
                .excluded_scopes()
                .iter()
                .any(|scope| scope.contains_path(&path))
            {
                return Err(CodingEndpointStartupError::Source);
            }
        }
        Ok(())
    }

    /// Transfer the exact policy, workspace and real offline worker into the
    /// existing retained coordinator owner. Source checks precede composition;
    /// original context validation, endpoint selection and capacity checks still
    /// apply. No model/tool dispatch occurs; each later advance needs separate
    /// disclosure consent and every file effect still asks for its exact grant.
    /// No coordinator recovery, native model qualification or executable service
    /// activation is established by this library composition.
    pub fn admit<'config, I: CodingIdentitySource, V: RuntimeVerifierPort, C: RuntimeClock>(
        self,
        runs: &mut CodingEndpointRuns<'config, CodingEndpointFiles<I>, V, C>,
        input: CodingEndpointStartInput<'config, '_, I, V, C>,
    ) -> Result<RuntimeEndpointRunRequest, CodingEndpointStartupError> {
        self.check_sources(input.sources)?;
        let request = self.bound.request.clone();
        let tool_boundary = CodingEndpointFiles::new(CodingEndpointFilesInput {
            bound: self.bound,
            workspace: self.workspace,
            authority: input.authority,
            sandbox: input.sandbox,
            registry: workspace_file_read_runtime_registry()
                .map_err(|_| CodingEndpointStartupError::Binding)?,
            actor_id: self.actor_id,
            identities: input.identities,
        })
        .map_err(|_| CodingEndpointStartupError::Native)?;
        runs.admit(CodingEndpointAdmission {
            request: request.clone(),
            connection: input.connection,
            registry: self.registry,
            sources: input.sources,
            source_budget: input.source_budget,
            include_usage: input.include_usage,
            tool_boundary,
            verifier: input.verifier,
            clock: input.clock,
        })
        .map_err(CodingEndpointStartupError::Owner)?;
        Ok(request)
    }
}

#[cfg(test)]
#[path = "coding_endpoint_startup_tests.rs"]
mod tests;
