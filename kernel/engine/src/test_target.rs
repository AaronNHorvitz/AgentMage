use agentmage_kernel_contracts::{GrantPreimage, GrantTarget};

pub(crate) fn target(path: &[&str]) -> GrantTarget {
    target_for(path, "workspace-0001", "authorization-0001", "adapter-0001")
}

pub(crate) fn target_for(
    path: &[&str],
    workspace_id: &str,
    authorization_id: &str,
    adapter_instance_id: &str,
) -> GrantTarget {
    serde_json::from_value(serde_json::json!({
        "target_kind": "held_object",
        "path": {"workspace_id": workspace_id, "components": path},
        "authorization_id": authorization_id,
        "adapter_instance_id": adapter_instance_id,
        "platform": "deterministic_fake",
        "object_kind": "regular_file",
        "object_identity": {
            "platform": "deterministic_fake",
            "mount_identity_sha256": vec![1_u8; 32],
            "object_identity_sha256": vec![2_u8; 32]
        },
        "preimage": {"byte_len": 7, "content_sha256": vec![3_u8; 32]}
    }))
    .expect("exact synthetic held target")
}

pub(crate) fn scope(path: &[&str]) -> GrantTarget {
    scope_for(path, "workspace-0001", "authorization-0001", "adapter-0001")
}

pub(crate) fn scope_for(
    path: &[&str],
    workspace_id: &str,
    authorization_id: &str,
    adapter_instance_id: &str,
) -> GrantTarget {
    serde_json::from_value(serde_json::json!({
        "target_kind": "workspace_scope",
        "path": {"workspace_id": workspace_id, "components": path},
        "authorization_id": authorization_id,
        "adapter_instance_id": adapter_instance_id,
        "platform": "deterministic_fake"
    }))
    .expect("synthetic workspace scope")
}

pub(crate) fn preimage(target_index: u32, target: &GrantTarget) -> GrantPreimage {
    GrantPreimage::for_target(target_index, target).expect("exact synthetic file preimage")
}
