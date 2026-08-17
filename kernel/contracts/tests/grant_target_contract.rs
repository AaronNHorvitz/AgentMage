use agentmage_kernel_contracts::{
    AdapterInstanceId, AuthorizedWorkspaceHandle, FilePreimage, GrantPreimage, GrantTarget,
    HeldWorkspaceObject, HeldWorkspaceRoot, PathPlatform, PathResolutionIntent,
    WorkspaceAuthorizationId, WorkspaceId, WorkspaceObjectIdentity, WorkspaceObjectKind,
    WorkspacePath, WorkspaceScopePath,
};

#[derive(Debug)]
struct FakeWorkspace {
    workspace_id: WorkspaceId,
    authorization_id: WorkspaceAuthorizationId,
    adapter_instance_id: AdapterInstanceId,
    root_identity: WorkspaceObjectIdentity,
}

impl HeldWorkspaceRoot for FakeWorkspace {
    fn root_identity(&self) -> &WorkspaceObjectIdentity {
        &self.root_identity
    }
}

impl AuthorizedWorkspaceHandle for FakeWorkspace {
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
struct FakeHeldObject {
    path: WorkspacePath,
    authorization_id: WorkspaceAuthorizationId,
    adapter_instance_id: AdapterInstanceId,
    identity: WorkspaceObjectIdentity,
    preimage: FilePreimage,
}

impl HeldWorkspaceObject for FakeHeldObject {
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
        PathResolutionIntent::ReadFile
    }

    fn object_kind(&self) -> WorkspaceObjectKind {
        WorkspaceObjectKind::RegularFile
    }

    fn object_identity(&self) -> &WorkspaceObjectIdentity {
        &self.identity
    }

    fn preimage(&self) -> Option<&FilePreimage> {
        Some(&self.preimage)
    }
}

fn workspace() -> FakeWorkspace {
    FakeWorkspace {
        workspace_id: WorkspaceId::from_raw("workspace-0001"),
        authorization_id: WorkspaceAuthorizationId::from_raw("authorization-0001"),
        adapter_instance_id: AdapterInstanceId::from_raw("adapter-0001"),
        root_identity: WorkspaceObjectIdentity::new(
            PathPlatform::DeterministicFake,
            [4; 32],
            [5; 32],
        ),
    }
}

#[test]
fn held_workspace_root_is_an_exact_root_operation_target() {
    let workspace = workspace();
    let scope = GrantTarget::workspace_scope(
        &workspace,
        WorkspaceScopePath::new(workspace.workspace_id.clone(), std::iter::empty::<&str>())
            .expect("root scope"),
    )
    .expect("bound root scope");
    let target = GrantTarget::held_workspace_root(&workspace).expect("held root target");

    assert!(scope.contains(&target));
    assert!(target.is_operation_target());
    assert!(target.scope_path().is_none());
    assert!(target.workspace_path().is_none());
    assert!(target.path_components().is_empty());
    assert_eq!(target.object_kind(), Some(WorkspaceObjectKind::Directory));
    assert_eq!(target.object_identity(), Some(&workspace.root_identity));
    assert!(target.preimage().is_none());
    assert!(GrantPreimage::for_target(0, &target).is_none());
    assert!(target.matches_held_workspace_root(&workspace));

    let excluded_child = GrantTarget::workspace_scope(
        &workspace,
        WorkspaceScopePath::new(workspace.workspace_id.clone(), ["private"])
            .expect("excluded child scope"),
    )
    .expect("bound child scope");
    assert!(excluded_child.overlaps_operation(&target));

    let encoded = serde_json::to_value(&target).expect("serialize root target");
    let decoded: GrantTarget = serde_json::from_value(encoded.clone()).expect("root target");
    assert_eq!(decoded, target);

    let mut non_root = encoded;
    non_root["path"]["components"] = serde_json::json!(["child"]);
    assert!(serde_json::from_value::<GrantTarget>(non_root).is_err());
}

fn held() -> FakeHeldObject {
    FakeHeldObject {
        path: WorkspacePath::new(WorkspaceId::from_raw("workspace-0001"), ["src", "lib.rs"])
            .expect("canonical path"),
        authorization_id: WorkspaceAuthorizationId::from_raw("authorization-0001"),
        adapter_instance_id: AdapterInstanceId::from_raw("adapter-0001"),
        identity: WorkspaceObjectIdentity::new(PathPlatform::DeterministicFake, [1; 32], [2; 32]),
        preimage: FilePreimage::new(7, [3; 32]),
    }
}

#[test]
fn scope_and_object_targets_reuse_canonical_path_types_without_reparsing() {
    let workspace = workspace();
    let root = GrantTarget::workspace_scope(
        &workspace,
        WorkspaceScopePath::new(workspace.workspace_id.clone(), std::iter::empty::<&str>())
            .expect("root scope"),
    )
    .expect("bound root");
    let held = held();
    let target = GrantTarget::held_object(&held).expect("held target");
    assert!(root.contains(&target));
    assert_eq!(target.workspace_path(), Some(&held.path));
    assert!(target.matches_held_object(&held));

    let preimage = GrantPreimage::for_target(0, &target).expect("file preimage");
    assert!(preimage.matches_target(0, &target));
    let encoded = serde_json::to_vec(&target).expect("serialize target");
    let decoded: GrantTarget = serde_json::from_slice(&encoded).expect("deserialize target");
    assert_eq!(decoded, target);
}

#[test]
fn every_workspace_path_rejection_is_also_a_grant_target_rejection() {
    let rejected = [
        "",
        ".",
        "..",
        "*",
        "**",
        "/rooted",
        "\\rooted",
        "drive:c",
        "a/b",
        "a\\b",
        "%2f",
        "%2e%2e",
        "trailing.",
        "trailing ",
        "e\u{301}",
        "hidden\u{200b}name",
    ];
    for component in rejected {
        let path = serde_json::json!({
            "workspace_id": "workspace-0001",
            "components": [component]
        });
        assert!(serde_json::from_value::<WorkspacePath>(path.clone()).is_err());
        let target = serde_json::json!({
            "target_kind": "held_object",
            "path": path,
            "authorization_id": "authorization-0001",
            "adapter_instance_id": "adapter-0001",
            "platform": "deterministic_fake",
            "object_kind": "regular_file",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": vec![2_u8; 32]
            },
            "preimage": {"byte_len": 7, "content_sha256": vec![3_u8; 32]}
        });
        assert!(serde_json::from_value::<GrantTarget>(target).is_err());
    }
}

#[test]
fn authorization_platform_identity_and_preimage_drift_change_the_exact_target() {
    let original = held();
    let target = GrantTarget::held_object(&original).expect("original target");

    let mut changed_authorization = held();
    changed_authorization.authorization_id =
        WorkspaceAuthorizationId::from_raw("authorization-0002");
    assert!(!target.matches_held_object(&changed_authorization));

    let mut changed_identity = held();
    changed_identity.identity =
        WorkspaceObjectIdentity::new(PathPlatform::DeterministicFake, [1; 32], [9; 32]);
    assert!(!target.matches_held_object(&changed_identity));

    let mut changed_preimage = held();
    changed_preimage.preimage = FilePreimage::new(7, [8; 32]);
    assert!(!target.matches_held_object(&changed_preimage));
}

#[test]
fn scope_constructor_rejects_a_path_from_another_workspace_handle() {
    let workspace = workspace();
    let foreign_path = WorkspaceScopePath::new(WorkspaceId::from_raw("workspace-foreign"), ["src"])
        .expect("canonical foreign scope");
    assert!(GrantTarget::workspace_scope(&workspace, foreign_path).is_err());
}

#[test]
fn held_target_wire_requires_every_binding_and_rejects_cross_platform_identity() {
    let target = GrantTarget::held_object(&held()).expect("held target");
    let value = serde_json::to_value(target).expect("target JSON");
    for key in [
        "target_kind",
        "path",
        "authorization_id",
        "adapter_instance_id",
        "platform",
        "object_kind",
        "object_identity",
        "preimage",
    ] {
        let mut missing = value.clone();
        missing.as_object_mut().expect("target object").remove(key);
        assert!(
            serde_json::from_value::<GrantTarget>(missing).is_err(),
            "missing {key}"
        );
    }

    let mut mismatched = value;
    mismatched["platform"] = serde_json::json!("linux");
    assert!(serde_json::from_value::<GrantTarget>(mismatched).is_err());
}
