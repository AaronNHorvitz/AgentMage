// SPDX-License-Identifier: Apache-2.0
//! Private immutable request storage; common fields never replace the original sealed request.

use super::{RuntimeLoopError, RuntimeRunScope, event_frame::RuntimeEventScope};
use agentmage_kernel_contracts::*;
use std::ops::Deref;

#[derive(Clone)]
enum Original {
    Native(Box<RuntimeRunRequest>),
    Endpoint(Box<RuntimeEndpointRunRequest>),
}

#[derive(Clone)]
pub(super) struct CoordinatorRequest {
    original: Original,
    fields: CommonRunFields,
}

#[derive(Clone)]
pub(super) struct CommonRunFields {
    pub run_id: RuntimeRunId,
    pub session_id: SessionId,
    pub task: Task,
    pub work_packet: WorkPacket,
    pub repository_snapshot_id: RepositorySnapshotId,
    pub tool_catalog_id: ToolCatalogId,
    pub tool_catalog_sha256: String,
    pub visible_tools: Vec<RuntimeToolReference>,
    pub limits: RuntimeRunLimits,
    pub request_sha256: String,
    pub mode: RuntimeSessionMode,
    pub event_cursor: Option<RuntimeEventCursor>,
}

impl From<RuntimeRunRequest> for CoordinatorRequest {
    fn from(request: RuntimeRunRequest) -> Self {
        let fields = CommonRunFields {
            run_id: request.run_id.clone(),
            session_id: request.session_id.clone(),
            task: request.task.clone(),
            work_packet: request.work_packet.clone(),
            repository_snapshot_id: request.repository_snapshot_id.clone(),
            tool_catalog_id: request.tool_catalog_id.clone(),
            tool_catalog_sha256: request.tool_catalog_sha256.clone(),
            visible_tools: request.visible_tools.clone(),
            limits: request.limits.clone(),
            request_sha256: request.request_sha256.clone(),
            mode: request.mode,
            event_cursor: request.event_cursor.clone(),
        };
        Self {
            original: Original::Native(Box::new(request)),
            fields,
        }
    }
}

impl From<RuntimeEndpointRunRequest> for CoordinatorRequest {
    fn from(request: RuntimeEndpointRunRequest) -> Self {
        let fields = CommonRunFields {
            run_id: request.run_id.clone(),
            session_id: request.session_id.clone(),
            task: request.task.clone(),
            work_packet: request.work_packet.clone(),
            repository_snapshot_id: request.repository_snapshot_id.clone(),
            tool_catalog_id: request.tool_catalog_id.clone(),
            tool_catalog_sha256: request.tool_catalog_sha256.clone(),
            visible_tools: request.visible_tools.clone(),
            limits: request.limits.clone(),
            request_sha256: request.request_sha256.clone(),
            mode: request.mode,
            event_cursor: request.event_cursor.clone(),
        };
        Self {
            original: Original::Endpoint(Box::new(request)),
            fields,
        }
    }
}

impl Deref for CoordinatorRequest {
    type Target = CommonRunFields;
    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

impl CoordinatorRequest {
    pub fn verify(&self) -> Result<(), RuntimeLoopError> {
        match &self.original {
            Original::Native(request) => {
                crate::runtime_coordinator::verify_runtime_run_request(request)
            }
            Original::Endpoint(request) => {
                crate::runtime_coordinator::verify_runtime_endpoint_run_request(request)
            }
        }
        .map_err(RuntimeLoopError::from)
    }

    pub fn native(&self) -> Result<&RuntimeRunRequest, RuntimeLoopError> {
        match &self.original {
            Original::Native(request) => Ok(request),
            Original::Endpoint(_) => Err(RuntimeLoopError::UnsupportedMode),
        }
    }
    pub fn endpoint(&self) -> Option<&RuntimeEndpointRunRequest> {
        match &self.original {
            Original::Endpoint(request) => Some(request),
            Original::Native(_) => None,
        }
    }
    pub fn scope(&self) -> Result<RuntimeRunScope<'_>, RuntimeLoopError> {
        match &self.original {
            Original::Native(request) => RuntimeRunScope::native(request),
            Original::Endpoint(request) => RuntimeRunScope::endpoint(request),
        }
        .map_err(RuntimeLoopError::Dependency)
    }
    pub fn event_scope(&self) -> RuntimeEventScope<'_> {
        match &self.original {
            Original::Native(request) => RuntimeEventScope::native(request),
            Original::Endpoint(request) => RuntimeEventScope::endpoint(request),
        }
    }
}

impl serde::Serialize for CoordinatorRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self.original {
            Original::Native(request) => request.serialize(serializer),
            Original::Endpoint(request) => request.serialize(serializer),
        }
    }
}
