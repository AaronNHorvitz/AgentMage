use std::sync::atomic::{AtomicUsize, Ordering};

use agentmage_kernel_contracts::{
    AdapterInstanceId, AuthorizedWorkspaceHandle, FilePreimage, HeldWorkspaceObject,
    PathAdapterError, PathAdapterErrorKind, PathPlatform, PathResolutionIntent,
    PlatformPathAdapter, WorkspaceAuthorizationId, WorkspaceId, WorkspaceObjectIdentity,
    WorkspaceObjectKind, WorkspacePath,
};

#[derive(Debug)]
struct ClientHandle {
    workspace_id: WorkspaceId,
    authorization_id: WorkspaceAuthorizationId,
    adapter_instance_id: AdapterInstanceId,
}

impl AuthorizedWorkspaceHandle for ClientHandle {
    fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    fn authorization_id(&self) -> &WorkspaceAuthorizationId {
        &self.authorization_id
    }

    fn adapter_instance_id(&self) -> &AdapterInstanceId {
        &self.adapter_instance_id
    }

    fn platform(&self) -> PathPlatform {
        PathPlatform::DeterministicFake
    }
}

#[derive(Debug)]
struct ClientHeldObject {
    path: WorkspacePath,
    authorization_id: WorkspaceAuthorizationId,
    adapter_instance_id: AdapterInstanceId,
    intent: PathResolutionIntent,
    object_identity: WorkspaceObjectIdentity,
}

impl HeldWorkspaceObject for ClientHeldObject {
    fn workspace_path(&self) -> &WorkspacePath {
        &self.path
    }

    fn authorization_id(&self) -> &WorkspaceAuthorizationId {
        &self.authorization_id
    }

    fn adapter_instance_id(&self) -> &AdapterInstanceId {
        &self.adapter_instance_id
    }

    fn intent(&self) -> PathResolutionIntent {
        self.intent
    }

    fn object_kind(&self) -> WorkspaceObjectKind {
        WorkspaceObjectKind::RegularFile
    }

    fn object_identity(&self) -> &WorkspaceObjectIdentity {
        &self.object_identity
    }

    fn preimage(&self) -> Option<&FilePreimage> {
        None
    }
}

#[derive(Debug)]
struct ClientAdapter {
    adapter_instance_id: AdapterInstanceId,
    observations: AtomicUsize,
}

impl PlatformPathAdapter for ClientAdapter {
    type WorkspaceHandle = ClientHandle;
    type HeldObject = ClientHeldObject;

    fn adapter_instance_id(&self) -> &AdapterInstanceId {
        &self.adapter_instance_id
    }

    fn platform(&self) -> PathPlatform {
        PathPlatform::DeterministicFake
    }

    fn resolve(
        &self,
        workspace: &Self::WorkspaceHandle,
        path: &WorkspacePath,
        intent: PathResolutionIntent,
    ) -> Result<Self::HeldObject, PathAdapterError> {
        if workspace.adapter_instance_id() != self.adapter_instance_id() {
            return Err(PathAdapterError::new(
                PathAdapterErrorKind::ForeignHandle,
                None,
            ));
        }
        if workspace.workspace_id() != path.workspace_id() {
            return Err(PathAdapterError::new(
                PathAdapterErrorKind::WorkspaceMismatch,
                None,
            ));
        }
        self.observations.fetch_add(1, Ordering::SeqCst);
        Ok(ClientHeldObject {
            path: path.clone(),
            authorization_id: workspace.authorization_id().clone(),
            adapter_instance_id: workspace.adapter_instance_id().clone(),
            intent,
            object_identity: WorkspaceObjectIdentity::new(
                PathPlatform::DeterministicFake,
                [1; 32],
                [2; 32],
            ),
        })
    }
}

fn handle(workspace_id: &str, adapter_instance_id: &str) -> ClientHandle {
    ClientHandle {
        workspace_id: WorkspaceId::from_raw(workspace_id),
        authorization_id: WorkspaceAuthorizationId::from_raw("authorization-0001"),
        adapter_instance_id: AdapterInstanceId::from_raw(adapter_instance_id),
    }
}

#[test]
fn public_adapter_contract_preserves_handle_affinity_and_zero_observation_denials() {
    let adapter = ClientAdapter {
        adapter_instance_id: AdapterInstanceId::from_raw("adapter-fake-0001"),
        observations: AtomicUsize::new(0),
    };
    let path = WorkspacePath::new(
        WorkspaceId::from_raw("workspace-0001"),
        ["fixtures", "input.txt"],
    )
    .expect("canonical path");
    let held = adapter
        .resolve(
            &handle("workspace-0001", "adapter-fake-0001"),
            &path,
            PathResolutionIntent::ReadFile,
        )
        .expect("matching handle");
    assert_eq!(held.workspace_path(), &path);
    assert_eq!(held.intent(), PathResolutionIntent::ReadFile);
    assert_eq!(held.object_kind(), WorkspaceObjectKind::RegularFile);
    assert_eq!(held.object_identity().object_identity_sha256(), &[2; 32]);
    assert!(held.preimage().is_none());
    assert_eq!(adapter.observations.load(Ordering::SeqCst), 1);

    let mismatched = WorkspacePath::new(
        WorkspaceId::from_raw("workspace-0002"),
        ["fixtures", "input.txt"],
    )
    .expect("canonical path");
    let workspace_error = adapter
        .resolve(
            &handle("workspace-0001", "adapter-fake-0001"),
            &mismatched,
            PathResolutionIntent::Metadata,
        )
        .expect_err("mismatched workspace");
    assert_eq!(
        workspace_error.kind(),
        PathAdapterErrorKind::WorkspaceMismatch
    );

    let foreign_error = adapter
        .resolve(
            &handle("workspace-0001", "adapter-foreign-0001"),
            &path,
            PathResolutionIntent::Metadata,
        )
        .expect_err("foreign handle");
    assert_eq!(foreign_error.kind(), PathAdapterErrorKind::ForeignHandle);
    assert_eq!(adapter.observations.load(Ordering::SeqCst), 1);
    assert_eq!(foreign_error.to_string(), "path.adapter.foreign_handle");
}
