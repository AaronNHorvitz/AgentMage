// SPDX-License-Identifier: MIT OR Apache-2.0
//! Borrowed, verified run scope for model-independent tool and verifier boundaries.

use agentmage_kernel_contracts::{
    PolicyId, RepositorySnapshotId, RuntimeEndpointRunRequest, RuntimeModelBinding, RuntimeRunId,
    RuntimeRunLimits, RuntimeRunRequest, RuntimeSessionMode, RuntimeToolReference, SessionId, Task,
    WorkPacket, WorkspaceId,
};

use super::RuntimePortFailure;

#[derive(Clone, Copy)]
enum Request<'a> {
    Native(&'a RuntimeRunRequest),
    Endpoint(&'a RuntimeEndpointRunRequest),
}

/// Complete sealed request borrowed without converting selected endpoints into
/// native model profiles. Construction validates the original digest and shape;
/// private storage prevents replacing fields after validation. This descriptive
/// view grants no disclosure, tool, effect, persistence or completion authority.
#[derive(Clone, Copy)]
pub struct RuntimeRunScope<'a>(Request<'a>);

macro_rules! field {
    ($name:ident, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $name(self) -> &'a $ty {
            match self.0 {
                Request::Native(request) => &request.$name,
                Request::Endpoint(request) => &request.$name,
            }
        }
    };
}

impl<'a> RuntimeRunScope<'a> {
    /// Validates the complete native run, retaining its original exact profile.
    pub fn native(request: &'a RuntimeRunRequest) -> Result<Self, RuntimePortFailure> {
        crate::runtime_coordinator::verify_runtime_run_request(request)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        Ok(Self(Request::Native(request)))
    }

    /// Validates endpoint framing without asserting a tokenizer or native model.
    /// Admission here does not enable a runtime mode or an effect boundary.
    pub fn endpoint(request: &'a RuntimeEndpointRunRequest) -> Result<Self, RuntimePortFailure> {
        crate::runtime_coordinator::verify_runtime_endpoint_run_request(request)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        Ok(Self(Request::Endpoint(request)))
    }

    /// Legacy native-only boundaries must explicitly require the original native
    /// request. Endpoint scopes refuse before invoking any such boundary.
    pub fn native_request(self) -> Result<&'a RuntimeRunRequest, RuntimePortFailure> {
        match self.0 {
            Request::Native(request) => Ok(request),
            Request::Endpoint(_) => Err(RuntimePortFailure::Invalid),
        }
    }

    /// Original endpoint request, absent for a native run. No reverse conversion.
    #[must_use]
    pub fn endpoint_request(self) -> Option<&'a RuntimeEndpointRunRequest> {
        match self.0 {
            Request::Native(_) => None,
            Request::Endpoint(request) => Some(request),
        }
    }

    /// The exact identity available at this boundary, with no fabricated profile.
    #[must_use]
    pub fn model_binding(self) -> RuntimeModelBinding<'a> {
        match self.0 {
            Request::Native(request) => RuntimeModelBinding::ExactProfile(&request.model_profile),
            Request::Endpoint(request) => RuntimeModelBinding::SelectedEndpoint {
                endpoint: &request.model_selection.endpoint,
                model: &request.model_selection.model,
            },
        }
    }

    /// Original authority and retention mode; this view never enables that mode.
    #[must_use]
    pub fn mode(self) -> RuntimeSessionMode {
        match self.0 {
            Request::Native(request) => request.mode,
            Request::Endpoint(request) => request.mode,
        }
    }

    field!(run_id, RuntimeRunId, "Exact run identity.");
    field!(session_id, SessionId, "Exact session identity.");
    field!(
        task,
        Task,
        "Complete original task, including its acceptance criteria."
    );
    field!(
        work_packet,
        WorkPacket,
        "Complete original work packet and budgets."
    );
    field!(workspace_id, WorkspaceId, "Authorized workspace identity.");
    field!(
        workspace_snapshot_sha256,
        str,
        "Exact workspace observation digest."
    );
    field!(
        repository_snapshot_id,
        RepositorySnapshotId,
        "Exact repository observation identity."
    );
    field!(
        repository_snapshot_sha256,
        str,
        "Exact repository observation digest."
    );
    field!(
        tool_catalog_id,
        agentmage_kernel_contracts::ToolCatalogId,
        "Frozen tool catalog identity."
    );
    field!(
        tool_catalog_sha256,
        str,
        "Complete frozen tool catalog digest."
    );
    field!(
        visible_tools,
        [RuntimeToolReference],
        "Complete ordered visible tool references."
    );
    field!(policy_id, PolicyId, "Exact deterministic policy identity.");
    field!(policy_sha256, str, "Complete policy revision digest.");
    field!(
        limits,
        RuntimeRunLimits,
        "Original run ceilings; no new resource ledger."
    );
    field!(
        request_sha256,
        str,
        "Digest of the complete original request, including model binding."
    );
}
