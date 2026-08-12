#![allow(dead_code)]

use agentmage_kernel_contracts::{GrantPreimage, GrantTarget};

pub fn target(path: &[&str]) -> GrantTarget {
    serde_json::from_value(serde_json::json!({
        "target_kind": "held_object",
        "path": {"workspace_id": "workspace-0001", "components": path},
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
    }))
    .expect("exact synthetic held target")
}

pub fn scope(path: &[&str]) -> GrantTarget {
    serde_json::from_value(serde_json::json!({
        "target_kind": "workspace_scope",
        "path": {"workspace_id": "workspace-0001", "components": path},
        "authorization_id": "authorization-0001",
        "adapter_instance_id": "adapter-0001",
        "platform": "deterministic_fake"
    }))
    .expect("synthetic workspace scope")
}

pub fn preimage(target_index: u32, target: &GrantTarget) -> GrantPreimage {
    GrantPreimage::for_target(target_index, target).expect("exact synthetic file preimage")
}
